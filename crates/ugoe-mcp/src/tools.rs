//! ツールの定義と実行。開いているプロジェクトはパスごとに持つ。
//!
//! 編集は提案 (`propose_edit`) と適用 (`apply_proposal`) に分ける (docs/ai-integration.md の
//! 「MCP サーバ」、ADR 0006)。提案は複製したドキュメントに適用して検証と差分を返すだけで、
//! 元のドキュメントは変えない。適用は提案の ID だけを受け取るので、承認する人は提案の中身と差分を
//! 見てから、MCP クライアントの確認 (Claude Code の許可の確認など) で適用を許す。
//! 提案の置き場は、エディタと同じ `ugoe_editor_core::Proposals` (一部のコマンドを外した採用もできる)。
//!
//! PSD からの自動リギングの判断を外部のエージェントに任せるツールは [`crate::autorig`]。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};
use ugoe_editor_core::{
    Command, Document, Project, ProjectMeta, ProposalError, ProposalView, Proposals, export_bundle,
};

use crate::autorig::{self, PsdSource};
use crate::diff;
use crate::render::{RenderRequest, Renderer};
use crate::schema::{CommandSchema, object_schema};
use crate::summary::{ELEMENT_KINDS, element, model_summary};

/// サーバの設定。
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// 読むだけにする: 適用・Undo・Redo・保存のツールを出さない。
    pub read_only: bool,
    /// 描画に使う wgpu のバックエンド (`vulkan`・`gl` など。カンマ区切り)。`None` なら
    /// 環境変数 `WGPU_BACKEND`、それも無ければ wgpu の既定。
    pub backend: Option<String>,
}

/// ツールの集まりと、開いているプロジェクト。
#[derive(Debug)]
pub struct Tools {
    options: Options,
    schema: CommandSchema,
    /// 絶対パス → 開いているプロジェクト。
    projects: BTreeMap<PathBuf, OpenProject>,
    /// `render_png` の GPU (最初の描画で用意して使い回す)。
    renderer: Renderer,
}

/// 開いているプロジェクト。
#[derive(Debug)]
pub(crate) struct OpenProject {
    meta: ProjectMeta,
    pub(crate) doc: Document,
    /// ドキュメントを変えるたび (適用・Undo・Redo) に増やす番号。
    revision: u64,
    /// 適用を待つ提案。提案は作ったときのドキュメントの状態に対するもので、ドキュメントが変わると
    /// 適用できない (Undo で同じ状態に戻れば、また使える)。
    proposals: Proposals,
    /// 自動リギングの元にする PSD と、レイヤーの分類 (`import_psd`・`get_psd_layers` で覚える)。
    pub(crate) psd: Option<PsdSource>,
}

impl OpenProject {
    pub(crate) fn new(project: Project) -> Self {
        Self {
            doc: project.document(),
            meta: project.meta,
            revision: 0,
            proposals: Proposals::new(),
            psd: None,
        }
    }

    fn project(&self) -> Project {
        Project::from_document(self.meta.clone(), &self.doc)
    }

    /// ドキュメントが変わった: 番号を進める。
    fn changed(&mut self) {
        self.revision += 1;
    }

    pub(crate) fn status(&self, path: &Path) -> Value {
        json!({
            "path": path,
            "revision": self.revision,
            "modified": self.doc.is_modified(),
            // create で作ってまだ保存していなければ false。
            "on_disk": path.join("project.json").is_file(),
        })
    }
}

/// ツールの結果: 構造化した値と、その後に添える中身 (画像と、画像の説明の文)。
pub(crate) struct Output {
    pub(crate) value: Value,
    pub(crate) content: Vec<Content>,
}

/// 構造化した値の後に添える中身。
pub(crate) enum Content {
    Text(String),
    Png(Vec<u8>),
}

impl From<Value> for Output {
    fn from(value: Value) -> Self {
        Self {
            value,
            content: Vec::new(),
        }
    }
}

pub(crate) type ToolResult = Result<Output, String>;

/// ツールの注釈 (MCP の `ToolAnnotations`)。クライアントが確認を求めるかの判断に使う。
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    /// 読むだけ。ドキュメントもディスクも変えない。
    Read,
    /// メモリ上のドキュメントを変える (Undo で戻せる)。
    Edit,
    /// ディスクに書く。
    Write,
}

impl Kind {
    pub(crate) fn annotations(self, title: &str) -> Value {
        let (read_only, destructive) = match self {
            Kind::Read => (true, false),
            Kind::Edit => (false, false),
            Kind::Write => (false, true),
        };
        json!({
            "title": title,
            "readOnlyHint": read_only,
            "destructiveHint": destructive,
            "idempotentHint": matches!(self, Kind::Read),
            "openWorldHint": false,
        })
    }
}

/// プロジェクトのパスの引数。
pub(crate) fn path_property() -> Value {
    json!({
        "type": "string",
        "description": "プロジェクト (.ugoeproj ディレクトリ) のパス。相対パスはサーバの作業ディレクトリから解決する。",
    })
}

/// `tools/list` の 1 つのツール。
pub(crate) fn tool_def(
    name: &str,
    title: &str,
    kind: Kind,
    description: &str,
    schema: Value,
) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": schema,
        "annotations": kind.annotations(title),
    })
}

impl Tools {
    pub fn new(options: Options) -> Self {
        Self {
            renderer: Renderer::new(options.backend.clone()),
            options,
            schema: CommandSchema::generate(),
            projects: BTreeMap::new(),
        }
    }

    /// `tools/list` の `tools`。
    pub fn list(&self) -> Vec<Value> {
        let tool = tool_def;
        let path_only = || object_schema(json!({ "path": path_property() }), &["path"], None);
        let mut tools = vec![
            tool(
                "list_proposals",
                "List proposals",
                Kind::Read,
                "適用を待っている提案の一覧 (提案の ID・題・理由・コマンドの要約)。stale=true の提案は、\
                 提案の後にモデルが変わったので適用できない (Undo で戻れば使える)。",
                path_only(),
            ),
            tool(
                "open_project",
                "Open project",
                Kind::Read,
                "プロジェクト (.ugoeproj) を開く。create=true なら新しい空のプロジェクトを作る \
                 (save_project までディスクには書かない)。開いているプロジェクトを開き直すと、\
                 ディスクの内容で置き換える (未保存の変更があれば discard_changes=true が要る)。",
                object_schema(
                    json!({
                        "path": path_property(),
                        "create": { "type": "boolean", "description": "新しい空のプロジェクトを作る" },
                        "name": { "type": "string", "description": "新しいプロジェクトの名前 (既定はディレクトリ名)" },
                        "discard_changes": { "type": "boolean", "description": "未保存の変更を捨てて開き直す" },
                    }),
                    &["path"],
                    None,
                ),
            ),
            tool(
                "close_project",
                "Close project",
                Kind::Read,
                "開いているプロジェクトを閉じる。未保存の変更があれば discard_changes=true が要る。",
                object_schema(
                    json!({
                        "path": path_property(),
                        "discard_changes": { "type": "boolean" },
                    }),
                    &["path"],
                    None,
                ),
            ),
            tool(
                "get_model_summary",
                "Model summary",
                Kind::Read,
                "モデルの要約: パラメータ (値域)・パーツ・デフォーマ・メッシュ (頂点と三角形の数)・\
                 グルー・テクスチャ・物理・モーション・表情と、各要素のキーフォーム格子の軸 \
                 (パラメータとキー) とセルの数。頂点の座標などの中身は get_element で読む。",
                path_only(),
            ),
            tool(
                "get_element",
                "Get element",
                Kind::Read,
                "要素 1 つの JSON (モデルのファイルでの形そのまま。キーフォームの中身を含む)。\
                 motion と expression は名前で、ほかは ID で指定する。",
                object_schema(
                    json!({
                        "path": path_property(),
                        "kind": { "type": "string", "enum": ELEMENT_KINDS },
                        "id": { "type": "string" },
                    }),
                    &["path", "kind", "id"],
                    None,
                ),
            ),
            tool(
                "propose_edit",
                "Propose edit",
                Kind::Read,
                "編集を提案する。commands (ugoe_editor_core::Command の列) を複製したモデルに 1 つの \
                 Undo 単位として適用し、検証の結果・ApplyReport (警告)・変わる要素の ID を返す。\
                 モデルは変えない。成功すると提案の ID を返すので、ユーザーの承認を得てから \
                 apply_proposal で適用する。失敗したら error を読んで直した提案を出し直す。",
                object_schema(
                    json!({
                        "path": path_property(),
                        "title": { "type": "string", "description": "提案の短い題" },
                        "rationale": { "type": "string", "description": "提案の理由 (人が読む)" },
                        "commands": {
                            "type": "array",
                            "items": CommandSchema::command_ref(),
                            "minItems": 1,
                        },
                    }),
                    &["path", "title", "commands"],
                    Some(&self.schema),
                ),
            ),
            tool(
                "render_png",
                "Render PNG",
                Kind::Read,
                "今のモデル (未保存の変更を含む) を指定したパラメータの値で描いた PNG を返す。\
                 `ugoe render` と同じ描画 (同じライブラリを呼ぶ)。params に無いパラメータは既定値。",
                object_schema(
                    json!({
                        "path": path_property(),
                        "params": {
                            "type": "object",
                            "description": "パラメータの ID → 値",
                            "additionalProperties": { "type": "number" },
                        },
                        "width": { "type": "integer", "minimum": 1, "maximum": 4096 },
                        "height": { "type": "integer", "minimum": 1, "maximum": 4096 },
                        "background": { "type": "string", "description": "transparent (既定)・white・black・#RRGGBB・#RRGGBBAA" },
                        "motion": { "type": "string", "description": "再生するモーションの名前" },
                        "expression": { "type": "string", "description": "使う表情の名前" },
                        "time": { "type": "number", "minimum": 0, "description": "この秒数だけ時間を進めてから描く (物理・モーション)" },
                    }),
                    &["path"],
                    None,
                ),
            ),
        ];
        tools.extend(autorig::tools());
        if !self.options.read_only {
            tools.extend([
                tool(
                    "apply_proposal",
                    "Apply proposal",
                    Kind::Edit,
                    "propose_edit・propose_rig で作った提案を適用する (1 回の undo で取り消せる)。\
                     ユーザーが提案を承認してから呼ぶ。提案の後にモデルが変わっていたら拒否する \
                     (提案し直す)。exclude で一部のコマンドを外せる (外したコマンドに頼るコマンドも \
                     外さないと適用できない。commands[].requires を見る)。ディスクには書かない。",
                    object_schema(
                        json!({
                            "path": path_property(),
                            "proposal": { "type": "string", "description": "propose_edit・propose_rig が返した提案の ID" },
                            "exclude": {
                                "type": "array",
                                "items": { "type": "integer", "minimum": 0 },
                                "description": "外すコマンドの番号 (提案の commands の 0 始まりの位置)",
                            },
                        }),
                        &["path", "proposal"],
                        None,
                    ),
                ),
                tool(
                    "undo",
                    "Undo",
                    Kind::Edit,
                    "直前の適用を取り消す。",
                    path_only(),
                ),
                tool(
                    "redo",
                    "Redo",
                    Kind::Edit,
                    "取り消した適用をやり直す。",
                    path_only(),
                ),
                tool(
                    "save_project",
                    "Save project",
                    Kind::Write,
                    "プロジェクトをディスクに保存する。bundle を指定すると .ugoe バンドルにも書き出す。",
                    object_schema(
                        json!({
                            "path": path_property(),
                            "bundle": { "type": "string", "description": "書き出す .ugoe のパス" },
                        }),
                        &["path"],
                        None,
                    ),
                ),
            ]);
        }
        tools
    }

    /// `tools/call` を実行する。未知のツールなら `None`。
    pub fn call(&mut self, name: &str, arguments: Value) -> Option<Value> {
        let edit_tool = matches!(name, "apply_proposal" | "undo" | "redo" | "save_project");
        let result = match name {
            "open_project" => parse(arguments).and_then(|a| self.open_project(a)),
            "close_project" => parse(arguments).and_then(|a| self.close_project(a)),
            "get_model_summary" => parse(arguments).and_then(|a| self.get_model_summary(a)),
            "get_element" => parse(arguments).and_then(|a| self.get_element(a)),
            "propose_edit" => parse(arguments).and_then(|a| self.propose_edit(a)),
            "render_png" => parse(arguments).and_then(|a| self.render_png(a)),
            "list_proposals" => parse(arguments).and_then(|a| self.list_proposals(a)),
            "import_psd" => parse(arguments).and_then(|a| self.import_psd(a)),
            "get_psd_layers" => parse(arguments).and_then(|a: autorig::LayersArgs| {
                let (_, open) = self.project(&a.path)?;
                autorig::get_psd_layers(open, a)
            }),
            "set_layer_labels" => parse(arguments).and_then(|a: autorig::LabelsArgs| {
                let (_, open) = self.project(&a.path)?;
                autorig::set_layer_labels(open, a)
            }),
            "get_rig_guide" => parse(arguments).and_then(|a: PathArgs| {
                let (_, open) = self.project(&a.path)?;
                autorig::get_rig_guide(open)
            }),
            "propose_rig" => parse(arguments).and_then(|a: autorig::RigArgs| {
                let (key, open) = self.project(&a.path)?;
                let proposal = autorig::propose_rig(open, a)?;
                add_proposal(&key, open, proposal)
            }),
            _ if edit_tool && self.options.read_only => Err(format!(
                "`{name}` is disabled (the server runs with --read-only)"
            )),
            "apply_proposal" => parse(arguments).and_then(|a| self.apply_proposal(a)),
            "undo" => parse(arguments).and_then(|a| self.undo(a, true)),
            "redo" => parse(arguments).and_then(|a| self.undo(a, false)),
            "save_project" => parse(arguments).and_then(|a| self.save_project(a)),
            _ => return None,
        };
        Some(tool_result(result))
    }

    /// パスを絶対パスにする (あれば正規化する)。開いているプロジェクトの鍵に使う。
    fn key(path: &str) -> Result<PathBuf, String> {
        let path = std::path::absolute(path).map_err(|e| format!("invalid path `{path}`: {e}"))?;
        Ok(path.canonicalize().unwrap_or(path))
    }

    /// 開いているプロジェクト。開いていなければディスクから開く (要求ごとに状態を渡す MCP の流儀に
    /// 合わせ、パスだけでプロジェクトを指せるようにする)。
    fn project(&mut self, path: &str) -> Result<(PathBuf, &mut OpenProject), String> {
        let key = Self::key(path)?;
        if !self.projects.contains_key(&key) {
            let project = Project::load(&key).map_err(|e| e.to_string())?;
            self.projects.insert(key.clone(), OpenProject::new(project));
        }
        match self.projects.get_mut(&key) {
            Some(open) => Ok((key, open)),
            None => Err(format!("project `{}` is not open", key.display())),
        }
    }

    fn open_project(&mut self, args: OpenArgs) -> ToolResult {
        let key = Self::key(&args.path)?;
        if let Some(open) = self.projects.get(&key)
            && open.doc.is_modified()
            && !args.discard_changes
        {
            return Err(format!(
                "`{}` has unsaved changes; save it or pass discard_changes=true",
                key.display()
            ));
        }
        let project = if args.create {
            if key.join("project.json").exists() {
                return Err(format!(
                    "`{}` is already a project; open it without create",
                    key.display()
                ));
            }
            let name = args.name.unwrap_or_else(|| {
                key.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "untitled".to_owned())
            });
            Project::empty(name)
        } else {
            Project::load(&key).map_err(|e| e.to_string())?
        };
        let open = OpenProject::new(project);
        let mut status = open.status(&key);
        let model = open.doc.model();
        status["name"] = json!(open.meta.name);
        status["counts"] = json!({
            "parameters": model.parameters.len(),
            "parts": model.parts.len(),
            "deformers": model.deformers.len(),
            "meshes": model.meshes.len(),
        });
        self.projects.insert(key, open);
        Ok(status.into())
    }

    fn close_project(&mut self, args: CloseArgs) -> ToolResult {
        let key = Self::key(&args.path)?;
        let Some(open) = self.projects.get(&key) else {
            return Err(format!("`{}` is not open", key.display()));
        };
        if open.doc.is_modified() && !args.discard_changes {
            return Err(format!(
                "`{}` has unsaved changes; save it or pass discard_changes=true",
                key.display()
            ));
        }
        self.projects.remove(&key);
        Ok(json!({ "closed": key }).into())
    }

    fn get_model_summary(&mut self, args: PathArgs) -> ToolResult {
        let (key, open) = self.project(&args.path)?;
        let mut summary = model_summary(&open.doc);
        summary["project"] = open.status(&key);
        Ok(summary.into())
    }

    fn get_element(&mut self, args: ElementArgs) -> ToolResult {
        let (_, open) = self.project(&args.path)?;
        match element(&open.doc, &args.kind, &args.id) {
            Some(value) => Ok(value.map_err(|e| e.to_string())?.into()),
            None if !ELEMENT_KINDS.contains(&args.kind.as_str()) => Err(format!(
                "unknown kind `{}` (expected one of {})",
                args.kind,
                ELEMENT_KINDS.join(", ")
            )),
            None => Err(format!("{} `{}` not found", args.kind, args.id)),
        }
    }

    fn propose_edit(&mut self, args: ProposeArgs) -> ToolResult {
        let (key, open) = self.project(&args.path)?;
        add_proposal(
            &key,
            open,
            Draft {
                title: args.title,
                rationale: args.rationale,
                commands: args.commands,
                extra: json!({}),
            },
        )
    }

    fn list_proposals(&mut self, args: PathArgs) -> ToolResult {
        let (key, open) = self.project(&args.path)?;
        let proposals: Vec<Value> = open
            .proposals
            .views(&open.doc)
            .iter()
            .map(proposal_json)
            .collect();
        Ok(json!({ "proposals": proposals, "project": open.status(&key) }).into())
    }

    fn apply_proposal(&mut self, args: ApplyArgs) -> ToolResult {
        let (key, open) = self.project(&args.path)?;
        let id = proposal_number(&args.proposal)?;
        let explain = |e: ProposalError| match e {
            ProposalError::Stale => format!(
                "proposal `{}` cannot be applied: the model changed after it was proposed (or the \
                 proposal was already applied); undo those changes or propose again",
                args.proposal
            ),
            ProposalError::NotFound(_) => format!("proposal `{}` not found", args.proposal),
            e => format!("the proposal was rejected: {e}"),
        };
        // 差分は、外したコマンドを除いた列を複製に適用して求める (承認した人が見るものと同じ)。
        let (after, _) = open
            .proposals
            .preview(&open.doc, id, &args.exclude)
            .map_err(explain)?;
        let changes = diff::diff(&open.doc, &after);
        let title = open
            .proposals
            .views(&open.doc)
            .into_iter()
            .find(|v| v.id == id)
            .map(|v| v.title)
            .unwrap_or_default();
        let report = open
            .proposals
            .adopt(&mut open.doc, id, &args.exclude)
            .map_err(explain)?;
        open.changed();
        Ok(json!({
            "applied": args.proposal,
            "title": title,
            "excluded": args.exclude,
            "report": report,
            "changes": changes,
            "project": open.status(&key),
        })
        .into())
    }

    fn import_psd(&mut self, args: autorig::ImportArgs) -> ToolResult {
        let key = Self::key(&args.path)?;
        if let Some(open) = self.projects.get(&key)
            && open.doc.is_modified()
            && !args.discard_changes
        {
            return Err(format!(
                "`{}` has unsaved changes; save it or pass discard_changes=true",
                key.display()
            ));
        }
        if key.join("project.json").exists() {
            return Err(format!(
                "`{}` is already a project; open it and pass `psd` to get_psd_layers instead",
                key.display()
            ));
        }
        let (open, value) = autorig::import_psd(args)?;
        let mut status = open.status(&key);
        if let (Value::Object(status), Value::Object(value)) = (&mut status, value) {
            status.extend(value);
        }
        self.projects.insert(key, open);
        Ok(status.into())
    }

    fn undo(&mut self, args: PathArgs, undo: bool) -> ToolResult {
        let (key, open) = self.project(&args.path)?;
        let done = if undo {
            open.doc.undo()
        } else {
            open.doc.redo()
        };
        if done {
            open.changed();
        }
        let mut status = open.status(&key);
        status[if undo { "undone" } else { "redone" }] = json!(done);
        Ok(status.into())
    }

    fn save_project(&mut self, args: SaveArgs) -> ToolResult {
        let (key, open) = self.project(&args.path)?;
        let project = open.project();
        project.save(&key).map_err(|e| e.to_string())?;
        open.doc.mark_saved();
        let mut status = open.status(&key);
        if let Some(bundle) = args.bundle {
            let bytes = export_bundle(&project).map_err(|e| e.to_string())?;
            std::fs::write(&bundle, bytes).map_err(|e| format!("failed to write {bundle}: {e}"))?;
            status["bundle"] = json!(std::path::absolute(&bundle).unwrap_or(bundle.into()));
        }
        Ok(status.into())
    }

    fn render_png(&mut self, args: RenderArgs) -> ToolResult {
        let (_, open) = self.project(&args.path)?;
        let project = open.project();
        let request = RenderRequest {
            params: args.params.into_iter().collect(),
            width: args.width,
            height: args.height,
            background: args.background,
            motion: args.motion,
            expression: args.expression,
            time: args.time,
        };
        let rendered = self
            .renderer
            .render(&project, &request)
            .map_err(|e| e.to_string())?;
        Ok(Output {
            value: json!({ "log": rendered.log, "bytes": rendered.png.len() }),
            content: vec![Content::Png(rendered.png)],
        })
    }
}

/// 置く前の提案。
pub(crate) struct Draft {
    pub(crate) title: String,
    pub(crate) rationale: String,
    pub(crate) commands: Vec<Command>,
    /// 結果に足す項目 (`propose_rig` の値など)。オブジェクト。
    pub(crate) extra: Value,
}

/// 提案を置き (複製に適用して検証する)、提案の ID・検証の結果・差分・コマンドの要約を返す。
/// モデルは変えない。
fn add_proposal(key: &Path, open: &mut OpenProject, draft: Draft) -> ToolResult {
    let rejected = |e: ProposalError| format!("the proposal was rejected: {e}");
    let view = open
        .proposals
        .add(&open.doc, draft.title, draft.rationale, draft.commands)
        .map_err(rejected)?;
    let (after, report) = open
        .proposals
        .preview(&open.doc, view.id, &[])
        .map_err(rejected)?;
    let changes = diff::diff(&open.doc, &after);
    let mut value = proposal_json(&view);
    value["report"] = json!(report);
    value["changes"] = changes;
    value["project"] = open.status(key);
    value["next"] =
        json!("ユーザーに提案 (題・理由・差分) を示し、承認を得てから apply_proposal で適用する");
    if let (Value::Object(value), Value::Object(extra)) = (&mut value, draft.extra) {
        value.extend(extra);
    }
    Ok(value.into())
}

/// 提案の ID の文字列 (`p3`)。
fn proposal_id(id: u32) -> String {
    format!("p{id}")
}

/// 提案の ID の文字列から番号を読む (`p3` か `3`)。
fn proposal_number(id: &str) -> Result<u32, String> {
    id.strip_prefix('p')
        .unwrap_or(id)
        .parse()
        .map_err(|_| format!("proposal `{id}` not found (proposal IDs look like `p1`)"))
}

fn proposal_json(view: &ProposalView) -> Value {
    json!({
        "proposal": proposal_id(view.id),
        "title": view.title,
        "rationale": view.rationale,
        "stale": view.stale,
        "commands": view.commands,
    })
}

/// ツールの結果 (`CallToolResult`) にする。失敗はツールの実行エラー (`isError`) として返し、
/// エージェントが読んで直せるようにする。
fn tool_result(result: ToolResult) -> Value {
    match result {
        Ok(output) => {
            let mut content = vec![json!({ "type": "text", "text": output.value.to_string() })];
            for item in &output.content {
                content.push(match item {
                    Content::Text(text) => json!({ "type": "text", "text": text }),
                    Content::Png(png) => json!({
                        "type": "image",
                        "data": base64::engine::general_purpose::STANDARD.encode(png),
                        "mimeType": "image/png",
                    }),
                });
            }
            json!({
                "content": content,
                "structuredContent": output.value,
                "isError": false,
            })
        }
        Err(message) => json!({
            "content": [{ "type": "text", "text": message }],
            "isError": true,
        }),
    }
}

fn parse<T: for<'de> Deserialize<'de>>(arguments: Value) -> Result<T, String> {
    serde_json::from_value(arguments).map_err(|e| format!("invalid arguments: {e}"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathArgs {
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenArgs {
    path: String,
    #[serde(default)]
    create: bool,
    name: Option<String>,
    #[serde(default)]
    discard_changes: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloseArgs {
    path: String,
    #[serde(default)]
    discard_changes: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementArgs {
    path: String,
    kind: String,
    id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposeArgs {
    path: String,
    title: String,
    #[serde(default)]
    rationale: String,
    commands: Vec<Command>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyArgs {
    path: String,
    proposal: String,
    #[serde(default)]
    exclude: Vec<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveArgs {
    path: String,
    bundle: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderArgs {
    path: String,
    #[serde(default)]
    params: BTreeMap<String, f32>,
    width: Option<u32>,
    height: Option<u32>,
    background: Option<String>,
    motion: Option<String>,
    expression: Option<String>,
    time: Option<f32>,
}
