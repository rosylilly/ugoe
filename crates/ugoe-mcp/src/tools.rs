//! ツールの定義と実行。プロジェクトの文書は持ち主 ([`DocumentHost`]) から借りる
//! (ファイルを自分で開く [`FileHost`] か、エディタ。`crate::host`)。
//!
//! 編集は提案 (`propose_edit`) と適用 (`apply_proposal`) に分ける (docs/ai-integration.md の
//! 「MCP サーバ」、ADR 0006)。提案は複製したドキュメントに適用して検証と差分を返すだけで、
//! 元のドキュメントは変えない。適用は提案の ID だけを受け取るので、承認する人は提案の中身と差分を
//! 見てから、MCP クライアントの確認 (Claude Code の許可の確認など) で適用を許す。エディタが持ち主なら、
//! 適用・保存などの前にエディタの確認も通す ([`DocumentHost::authorize`])。
//! 提案の置き場は、エディタと同じ `ugoe_editor_core::Proposals` (一部のコマンドを外した採用もできる)。
//!
//! PSD からの自動リギングの判断を外部のエージェントに任せるツールは [`crate::autorig`]。

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};
use ugoe_editor_core::{Command, Document, Project, ProposalError, ProposalView};

use crate::autorig::{self, PsdSource};
use crate::diff;
use crate::host::{Action, Change, DocumentHost, FileHost, HostError, Lent, Opening};
use crate::render::{RenderRequest, Renderer};
use crate::root::Root;
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
    /// ツールが触れるパスをこのディレクトリの下に限る (`--root`)。相対パスもここから解決する。
    /// `None` なら限らず、相対パスはサーバの作業ディレクトリから解決する。
    pub root: Option<Root>,
}

/// ツールの集まり。文書は持ち主から借り、ツールだけが使う状態 ([`Extra`]) をパスごとに持つ。
///
/// `&self` で呼べ、複数のスレッドから同時に呼んでよい (HTTP の接続ごとのスレッド)。持ち主の錠を
/// 握るのは [`Tools::with`] の中だけで、その中ではほかの錠を取らない (錠の順序で詰まらないように)。
pub struct Tools {
    options: Options,
    schema: CommandSchema,
    host: Arc<dyn DocumentHost>,
    /// 絶対パス → ツールだけが使う状態。
    extras: Mutex<BTreeMap<PathBuf, Extra>>,
    /// `render_png` の GPU (最初の描画で用意して使い回す)。
    renderer: Mutex<Renderer>,
    /// 最後に握手したクライアントの名前 (提案の出どころとして持ち主に知らせる)。
    client: Mutex<Option<String>>,
}

impl fmt::Debug for Tools {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tools")
            .field("options", &self.options)
            .field("extras", &self.extras)
            .finish_non_exhaustive()
    }
}

/// 覚えている PSD (プロジェクトごと。PSD の画像を扱う間は長く握るので、表とは別の錠)。
type PsdSlot = Arc<Mutex<Option<PsdSource>>>;

/// ツールだけが使う、プロジェクトごとの状態 (持ち主は知らない)。開き直すと消える。
#[derive(Debug, Default)]
struct Extra {
    /// ツールがドキュメントを変えるたび (適用・Undo・Redo) に増やす番号。
    revision: u64,
    /// 自動リギングの元にする PSD と、レイヤーの分類 (`import_psd`・`get_psd_layers` で覚える)。
    /// PSD の画像を扱う間は長く握るので、表とは別の錠にする。
    psd: PsdSlot,
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

/// パスの引数の説明に添える、解決の規則。
pub(crate) const PATH_NOTE: &str =
    "相対パスは --root (あれば) かサーバの作業ディレクトリから解決する。--root の外は使えない。";

/// プロジェクトのパスの引数。
pub(crate) fn path_property() -> Value {
    json!({
        "type": "string",
        "description": format!("プロジェクト (.ugoeproj ディレクトリ) のパス。{PATH_NOTE}"),
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

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // ツールの panic は描画の中で受け止めているが、万一毒されても状態は使い続ける。
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn host_error(e: HostError) -> String {
    e.to_string()
}

impl Tools {
    /// ファイルを自分で開く持ち主 ([`FileHost`]) で動かす。
    pub fn new(options: Options) -> Self {
        let host = Arc::new(FileHost::new(options.root.clone()));
        Self::with_host(options, host)
    }

    /// 外から渡した持ち主 (エディタなど) で動かす。
    pub fn with_host(options: Options, host: Arc<dyn DocumentHost>) -> Self {
        Self {
            renderer: Mutex::new(Renderer::new(options.backend.clone())),
            options,
            schema: CommandSchema::generate(),
            host,
            extras: Mutex::default(),
            client: Mutex::default(),
        }
    }

    /// 持ち主がサーバの `instructions` に足す文。
    pub fn host_instructions(&self) -> Option<String> {
        self.host.instructions()
    }

    /// クライアントの名前を覚え、握手 (`handshake`) か名前が変わったときに持ち主に知らせる
    /// (要求ごとに版を示す版では、要求のたびに名前が届く)。
    pub fn client_connected(&self, name: &str, version: Option<&str>, handshake: bool) {
        let previous = lock(&self.client).replace(name.to_owned());
        if handshake || previous.as_deref() != Some(name) {
            self.host
                .changed(&Change::ClientConnected { name, version });
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
                        "motion": { "type": "string", "description": format!("再生するモーションの名前か .motion.json のパス。{PATH_NOTE}") },
                        "expression": { "type": "string", "description": format!("使う表情の名前か .expression.json のパス。{PATH_NOTE}") },
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
                            "bundle": { "type": "string", "description": format!("書き出す .ugoe のパス。{PATH_NOTE}") },
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
    pub fn call(&self, name: &str, arguments: Value) -> Option<Value> {
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
                let psd = a.psd.as_deref().map(|p| self.resolve(p)).transpose()?;
                let key = self.resolve(&a.path)?;
                let (doc, slot) = self.for_autorig(&key)?;
                autorig::get_psd_layers(&doc, &mut lock(&slot), a, psd)
            }),
            "set_layer_labels" => parse(arguments).and_then(|a: autorig::LabelsArgs| {
                let key = self.resolve(&a.path)?;
                let (doc, slot) = self.for_autorig(&key)?;
                autorig::set_layer_labels(&doc, &mut lock(&slot), a)
            }),
            "get_rig_guide" => parse(arguments).and_then(|a: PathArgs| {
                let key = self.resolve(&a.path)?;
                let (doc, slot) = self.for_autorig(&key)?;
                autorig::get_rig_guide(&doc, &lock(&slot))
            }),
            "propose_rig" => parse(arguments).and_then(|a: autorig::RigArgs| {
                let key = self.resolve(&a.path)?;
                let (doc, slot) = self.for_autorig(&key)?;
                let draft = autorig::propose_rig(&doc, &lock(&slot), a)?;
                self.add_proposal(&key, draft)
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

    /// ツールが受け取ったパスを絶対パスにする (あれば正規化する)。開いているプロジェクトの鍵にも使う。
    /// `--root` があれば root から解決し、root の外ならエラー (`crate::root`)。
    pub(crate) fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        if let Some(root) = &self.options.root {
            return root.resolve(path);
        }
        let path = std::path::absolute(path).map_err(|e| format!("invalid path `{path}`: {e}"))?;
        Ok(path.canonicalize().unwrap_or(path))
    }

    /// 持ち主からプロジェクト `key` を借りて `f` を呼ぶ (開いていなければ持ち主が開くか断る。
    /// `FileHost` はディスクから読む。要求ごとに状態を渡す MCP の流儀に合わせ、パスだけで
    /// プロジェクトを指せるようにする)。`f` は持ち主の錠の中で動くので、短くし、ほかの錠を取らない。
    fn with<R>(&self, key: &Path, f: impl FnOnce(Lent<'_>) -> R) -> Result<R, String> {
        let mut f = Some(f);
        let mut out = None;
        self.host
            .with_project(key, &mut |lent| {
                if let Some(f) = f.take() {
                    out = Some(f(lent));
                }
            })
            .map_err(host_error)?;
        out.ok_or_else(|| format!("project `{}` is not open", key.display()))
    }

    fn authorize(&self, action: Action<'_>) -> Result<(), String> {
        self.host.authorize(&action).map_err(host_error)
    }

    fn extras(&self) -> MutexGuard<'_, BTreeMap<PathBuf, Extra>> {
        lock(&self.extras)
    }

    /// ツールがドキュメントを変えた: 番号を進め、持ち主に知らせる。
    fn edited(&self, key: &Path) {
        self.extras().entry(key.to_path_buf()).or_default().revision += 1;
        self.host.changed(&Change::Edited { project: key });
    }

    /// 開き直した・閉じた: ツールだけの状態を捨てる。
    fn forget(&self, key: &Path) {
        self.extras().remove(key);
    }

    /// プロジェクトの状態 (`modified` は借りたときの文書から)。
    fn status(&self, key: &Path, modified: bool) -> Value {
        let revision = self.extras().get(key).map_or(0, |e| e.revision);
        json!({
            "path": key,
            "revision": revision,
            "modified": modified,
            // create で作ってまだ保存していなければ false。
            "on_disk": key.join("project.json").is_file(),
        })
    }

    /// 自動リギングのツールの入力: 文書の複製と、覚えている PSD。PSD の画像は重いので、持ち主の錠の
    /// 外で扱う。
    fn for_autorig(&self, key: &Path) -> Result<(Document, PsdSlot), String> {
        let doc = self.with(key, |lent| lent.document.detached())?;
        let slot = Arc::clone(&self.extras().entry(key.to_path_buf()).or_default().psd);
        Ok((doc, slot))
    }

    fn open_project(&self, args: OpenArgs) -> ToolResult {
        let key = self.resolve(&args.path)?;
        if args.create && key.join("project.json").exists() {
            return Err(format!(
                "`{}` is already a project; open it without create",
                key.display()
            ));
        }
        self.authorize(Action::Open {
            project: &key,
            create: args.create,
            discard_changes: args.discard_changes,
        })?;
        let opening = if args.create {
            let name = args.name.unwrap_or_else(|| {
                key.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "untitled".to_owned())
            });
            let project = Project::empty(name);
            Opening::New {
                document: Box::new(project.document()),
                meta: project.meta,
            }
        } else {
            Opening::Load
        };
        self.host
            .open(&key, opening, args.discard_changes)
            .map_err(host_error)?;
        self.forget(&key);
        self.host.changed(&Change::Opened { project: &key });
        let (modified, name, counts) = self.with(&key, |lent| {
            let model = lent.document.model();
            (
                lent.document.is_modified(),
                lent.meta.name.clone(),
                json!({
                    "parameters": model.parameters.len(),
                    "parts": model.parts.len(),
                    "deformers": model.deformers.len(),
                    "meshes": model.meshes.len(),
                }),
            )
        })?;
        let mut status = self.status(&key, modified);
        status["name"] = json!(name);
        status["counts"] = counts;
        Ok(status.into())
    }

    fn close_project(&self, args: CloseArgs) -> ToolResult {
        let key = self.resolve(&args.path)?;
        self.authorize(Action::Close {
            project: &key,
            discard_changes: args.discard_changes,
        })?;
        self.host
            .close(&key, args.discard_changes)
            .map_err(host_error)?;
        self.forget(&key);
        self.host.changed(&Change::Closed { project: &key });
        Ok(json!({ "closed": key }).into())
    }

    fn get_model_summary(&self, args: PathArgs) -> ToolResult {
        let key = self.resolve(&args.path)?;
        let (mut summary, modified) = self.with(&key, |lent| {
            (model_summary(lent.document), lent.document.is_modified())
        })?;
        summary["project"] = self.status(&key, modified);
        Ok(summary.into())
    }

    fn get_element(&self, args: ElementArgs) -> ToolResult {
        let key = self.resolve(&args.path)?;
        let found = self.with(&key, |lent| {
            element(lent.document, &args.kind, &args.id).map(|v| v.map_err(|e| e.to_string()))
        })?;
        match found {
            Some(value) => Ok(value?.into()),
            None if !ELEMENT_KINDS.contains(&args.kind.as_str()) => Err(format!(
                "unknown kind `{}` (expected one of {})",
                args.kind,
                ELEMENT_KINDS.join(", ")
            )),
            None => Err(format!("{} `{}` not found", args.kind, args.id)),
        }
    }

    fn propose_edit(&self, args: ProposeArgs) -> ToolResult {
        let key = self.resolve(&args.path)?;
        self.add_proposal(
            &key,
            Draft {
                title: args.title,
                rationale: args.rationale,
                commands: args.commands,
                extra: json!({}),
            },
        )
    }

    fn list_proposals(&self, args: PathArgs) -> ToolResult {
        let key = self.resolve(&args.path)?;
        let (proposals, modified) = self.with(&key, |lent| {
            let proposals: Vec<Value> = lent
                .proposals
                .views(lent.document)
                .iter()
                .map(proposal_json)
                .collect();
            (proposals, lent.document.is_modified())
        })?;
        Ok(json!({ "proposals": proposals, "project": self.status(&key, modified) }).into())
    }

    fn apply_proposal(&self, args: ApplyArgs) -> ToolResult {
        let key = self.resolve(&args.path)?;
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
        // 確認を求める前に、適用できる提案かを確かめる (適用できない提案で利用者に尋ねないように)。
        let title = self.with(&key, |lent| {
            lent.proposals
                .preview(lent.document, id, &args.exclude)
                .map_err(explain)?;
            Ok::<_, String>(
                lent.proposals
                    .views(lent.document)
                    .into_iter()
                    .find(|v| v.id == id)
                    .map(|v| v.title)
                    .unwrap_or_default(),
            )
        })??;
        // 持ち主の確認 (エディタのダイアログ) を待つ間は、どの錠も握らない。
        self.authorize(Action::Apply {
            project: &key,
            proposal: id,
            title: &title,
            excluded: &args.exclude,
        })?;
        // 確認の間にモデルが変わっていれば、提案が古いというエラーになる (承認した差分と違う結果にしない)。
        let (report, changes, modified) = self.with(&key, |lent| {
            // 差分は、外したコマンドを除いた列を複製に適用して求める (承認した人が見るものと同じ)。
            let (after, _) = lent
                .proposals
                .preview(lent.document, id, &args.exclude)
                .map_err(explain)?;
            let changes = diff::diff(lent.document, &after);
            let report = lent
                .proposals
                .adopt(lent.document, id, &args.exclude)
                .map_err(explain)?;
            Ok::<_, String>((report, changes, lent.document.is_modified()))
        })??;
        self.edited(&key);
        Ok(json!({
            "applied": args.proposal,
            "title": title,
            "excluded": args.exclude,
            "report": report,
            "changes": changes,
            "project": self.status(&key, modified),
        })
        .into())
    }

    fn import_psd(&self, args: autorig::ImportArgs) -> ToolResult {
        let key = self.resolve(&args.path)?;
        if key.join("project.json").exists() {
            return Err(format!(
                "`{}` is already a project; open it and pass `psd` to get_psd_layers instead",
                key.display()
            ));
        }
        let psd = self.resolve(&args.psd)?;
        self.authorize(Action::Import {
            project: &key,
            psd: &psd,
            discard_changes: args.discard_changes,
        })?;
        let discard_changes = args.discard_changes;
        let imported = autorig::import_psd(args, psd)?;
        self.host
            .open(
                &key,
                Opening::New {
                    meta: imported.meta,
                    document: Box::new(imported.document),
                },
                discard_changes,
            )
            .map_err(host_error)?;
        self.forget(&key);
        *lock(&self.extras().entry(key.clone()).or_default().psd) = Some(imported.source);
        self.host.changed(&Change::Opened { project: &key });
        let modified = self.with(&key, |lent| lent.document.is_modified())?;
        let mut status = self.status(&key, modified);
        if let (Value::Object(status), Value::Object(value)) = (&mut status, imported.value) {
            status.extend(value);
        }
        Ok(status.into())
    }

    fn undo(&self, args: PathArgs, undo: bool) -> ToolResult {
        let key = self.resolve(&args.path)?;
        self.authorize(if undo {
            Action::Undo { project: &key }
        } else {
            Action::Redo { project: &key }
        })?;
        let (done, modified) = self.with(&key, |lent| {
            let done = if undo {
                lent.document.undo()
            } else {
                lent.document.redo()
            };
            (done, lent.document.is_modified())
        })?;
        if done {
            self.edited(&key);
        }
        let mut status = self.status(&key, modified);
        status[if undo { "undone" } else { "redone" }] = json!(done);
        Ok(status.into())
    }

    fn save_project(&self, args: SaveArgs) -> ToolResult {
        // 書く前にパスを確かめる (保存の後で書き出し先が root の外と分かって半端にならないように)。
        let bundle = args
            .bundle
            .as_deref()
            .map(|b| self.resolve(b))
            .transpose()?;
        let key = self.resolve(&args.path)?;
        self.authorize(Action::Save {
            project: &key,
            bundle: bundle.as_deref(),
        })?;
        self.host
            .save(&key, bundle.as_deref())
            .map_err(host_error)?;
        self.host.changed(&Change::Saved { project: &key });
        let modified = self.with(&key, |lent| lent.document.is_modified())?;
        let mut status = self.status(&key, modified);
        if let Some(bundle) = bundle {
            status["bundle"] = json!(bundle);
        }
        Ok(status.into())
    }

    /// `render_png` の `motion`・`expression`: プロジェクトの中の名前ならそのまま。そうでなければ
    /// JSON のファイルのパスとして解決する (`--root` の外なら拒否する。描画は解決したパスを読む)。
    fn name_or_path(
        &self,
        arg: Option<String>,
        names: &[&str],
        what: &str,
    ) -> Result<Option<String>, String> {
        let Some(arg) = arg else {
            return Ok(None);
        };
        if self.options.root.is_none() || names.contains(&arg.as_str()) {
            return Ok(Some(arg));
        }
        let unknown = format!("no {what} named `{arg}` (known: {})", names.join(", "));
        let path = self
            .resolve(&arg)
            .map_err(|e| format!("{unknown}, and it is not a usable path: {e}"))?;
        // 無いパスを元の文字列のまま渡すと、描画が作業ディレクトリから読もうとするので、ここで断る。
        if !path.is_file() {
            return Err(format!("{unknown}, and `{arg}` is not a file"));
        }
        Ok(Some(path.to_string_lossy().into_owned()))
    }

    fn render_png(&self, args: RenderArgs) -> ToolResult {
        let key = self.resolve(&args.path)?;
        // 描画は持ち主の錠の外で行う (エディタの画面を止めないように)。
        let project = self.with(&key, |lent| {
            Project::from_document(lent.meta.clone(), lent.document)
        })?;
        let motion_names: Vec<&str> = project.motions.iter().map(|m| m.name.as_str()).collect();
        let expression_names: Vec<&str> = project
            .expressions
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        let motion = self.name_or_path(args.motion, &motion_names, "motion")?;
        let expression = self.name_or_path(args.expression, &expression_names, "expression")?;
        let request = RenderRequest {
            params: args.params.into_iter().collect(),
            width: args.width,
            height: args.height,
            background: args.background,
            motion,
            expression,
            time: args.time,
        };
        let rendered = lock(&self.renderer)
            .render(&project, &request)
            .map_err(|e| e.to_string())?;
        Ok(Output {
            value: json!({ "log": rendered.log, "bytes": rendered.png.len() }),
            content: vec![Content::Png(rendered.png)],
        })
    }

    /// 提案を置き (複製に適用して検証する)、提案の ID・検証の結果・差分・コマンドの要約を返す。
    /// モデルは変えない。
    fn add_proposal(&self, key: &Path, draft: Draft) -> ToolResult {
        let rejected = |e: ProposalError| format!("the proposal was rejected: {e}");
        let (mut value, modified) = self.with(key, |lent| {
            let view = lent
                .proposals
                .add(lent.document, draft.title, draft.rationale, draft.commands)
                .map_err(rejected)?;
            let (after, report) = lent
                .proposals
                .preview(lent.document, view.id, &[])
                .map_err(rejected)?;
            let mut value = proposal_json(&view);
            value["report"] = json!(report);
            value["changes"] = diff::diff(lent.document, &after);
            Ok::<_, String>((value, lent.document.is_modified()))
        })??;
        value["project"] = self.status(key, modified);
        value["next"] = json!(
            "ユーザーに提案 (題・理由・差分) を示し、承認を得てから apply_proposal で適用する"
        );
        if let (Value::Object(value), Value::Object(extra)) = (&mut value, draft.extra) {
            value.extend(extra);
        }
        if let Some(id) = value["proposal"]
            .as_str()
            .and_then(|p| proposal_number(p).ok())
        {
            let client = lock(&self.client).clone();
            self.host.changed(&Change::Proposed {
                project: key,
                proposal: id,
                client: client.as_deref(),
            });
        }
        Ok(value.into())
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
