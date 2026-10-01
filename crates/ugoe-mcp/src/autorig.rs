//! PSD からの自動リギングの判断を、MCP クライアント側のエージェントに任せるツール
//! (ADR 0022 の「外部のエージェントに判断を任せる経路」、`docs/ai-integration.md#エージェントに任せる自動リギング`)。
//!
//! エディタの中で API を呼ぶ経路 (`ugoe_ai::classify::classify_layers`・`ugoe_ai::rig::propose_rig_with_ai`) と
//! 同じ段階を、ツールに分けて出す。画像を見て分類する・値を選ぶのはエージェントで、ugoe は API を呼ばない。
//!
//! 1. `import_psd`: PSD から新しいプロジェクトを作る (既存のプロジェクトなら `get_psd_layers` に `psd` を渡す)
//! 2. `get_psd_layers`: レイヤーの一覧・全体の合成画像・レイヤーのサムネイル (MCP の image コンテンツ) と、
//!    分類の指示と語彙。名前の規則による分類 (`classify_by_name`) を既定の案として付ける
//! 3. `set_layer_labels`: エージェントが決めた分類と差分の組 (`variant_sets`) を受け取る (形は
//!    `ugoe_ai::classify::output_schema`)
//! 4. `get_rig_guide`: テンプレートで決められない値 (`RigParams`) の説明・顔や目の位置・原画
//! 5. `propose_rig`: 値 (形は `ugoe_ai::rig::rig_output_schema`) からリグの提案を作る。範囲の外・検証を通らない
//!    値はツールのエラーで返し、エージェントが直して呼び直す
//!
//! 着せ替えのモジュールのプロジェクト (`import_psd` に `base` と `slot` を渡して作るか、エディタで作ったもの) では、
//! 4・5 の代わりに `get_module_rig_guide` と `propose_module_rig` を使う (`ugoe_ai::module_rig`。パラメータを足さず、
//! 衣装を本体に追従させ、揺らし、差分の組 (`variant_sets`) と名前・絵の重なりで見つけた差分をポーズにする)。
//!
//! 提案の適用は `apply_proposal` (利用者の確認を通す。ADR 0006)。画像の大きさと 1 回に返す枚数は、
//! API で分類するときと同じ上限 (`ClassifyOptions` の既定値) に合わせる。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};
use ugoe_ai::AiError;
use ugoe_ai::classify::{
    ClassifyOptions, Label, LayerLabel, apply_answer, classification_instructions,
    classify_by_name, layer_thumbnail_png, layers, output_schema, overview_png,
};
use ugoe_ai::module_rig::{
    FollowMode, ModuleRigGuidance, ModuleRigOptions, ModuleRigParams, module_mesh_parts,
    module_params_from_answer, module_rig_output_schema, module_rig_request_guided,
    propose_module_rig as template_module_rig,
};
use ugoe_ai::rig::{
    RigOptions, RigParams, mesh_parts, params_from_answer, propose_rig as template_rig,
    rig_output_schema, rig_request,
};
use ugoe_editor_core::import::layer_tree_command;
use ugoe_editor_core::{Document, Project, ProjectMeta, PsdImportOptions};
use ugoe_model::SlotId;
use ugoe_psd::LayerTree;

use crate::schema::object_schema;
use crate::tools::{Content, Draft, Kind, Output, PATH_NOTE, ToolResult, path_property, tool_def};

/// PSD がまだ分からないときの断りの文。
const NO_PSD: &str = "no PSD is associated with this project: call import_psd, or pass `psd` \
                      (the PSD this model was imported from) to get_psd_layers";

/// モジュールのプロジェクトで本体向けのリグのツールを呼んだときの断りの文。
const USE_MODULE_RIG: &str = "this is a module project, which cannot add parameters: use \
                              get_module_rig_guide and propose_module_rig instead";

/// モジュールのプロジェクトでないときにモジュールのリグのツールを呼んだときの断りの文。
const NOT_A_MODULE: &str = "this is not a module project: use get_rig_guide and propose_rig, or \
                            create a module project with import_psd and `base` and `slot`";

/// `get_psd_layers` が既定で 1 回に返すレイヤーの数。上限 (`ClassifyOptions::max_layers_per_request`) より
/// 小さくする: MCP クライアントは画像を含む結果の大きさに上限を設けることがある (Claude Code は既定で
/// 25,000 トークン。画像も数える) ので、既定では小分けにし、多く返すのは `count` で頼まれたときだけにする。
const DEFAULT_COUNT: usize = 8;

/// 自動リギングの元にする PSD と、レイヤーの分類。
pub(crate) struct PsdSource {
    path: PathBuf,
    tree: LayerTree,
    /// 分類 (`classify_by_name` と同じ並び)。エージェントが決めていないレイヤーは名前の規則の案。
    labels: Vec<LayerLabel>,
    /// エージェントが決めたレイヤーの番号。
    by_agent: BTreeSet<usize>,
}

impl fmt::Debug for PsdSource {
    // 画素を書き出さない。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PsdSource")
            .field("path", &self.path)
            .field("layers", &self.labels.len())
            .field("by_agent", &self.by_agent.len())
            .finish()
    }
}

impl PsdSource {
    fn load(path: PathBuf) -> Result<Self, String> {
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let tree = ugoe_psd::import(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self {
            labels: classify_by_name(&tree),
            by_agent: BTreeSet::new(),
            path,
            tree,
        })
    }
}

/// `get_psd_layers` の引数。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LayersArgs {
    pub(crate) path: String,
    pub(crate) psd: Option<String>,
    #[serde(default)]
    start: usize,
    count: Option<usize>,
    indices: Option<Vec<usize>>,
    #[serde(default = "yes")]
    images: bool,
    overview: Option<bool>,
    thumbnail_size: Option<u32>,
}

fn yes() -> bool {
    true
}

/// `set_layer_labels` の引数。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LabelsArgs {
    pub(crate) path: String,
    /// `output_schema` の `layers` の形 (ugoe-ai が読む)。省略すると分類を変えない。
    #[serde(default)]
    layers: Option<Value>,
    /// `output_schema` の `variant_sets` の形 (同じ部品の別の絵のレイヤーの組)。渡すと PSD 全体の組を置き換える。
    #[serde(default)]
    variant_sets: Option<Value>,
    #[serde(default)]
    reset: bool,
}

/// `propose_rig` の引数。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RigArgs {
    pub(crate) path: String,
    /// `rig_output_schema` の形 (ugoe-ai が読む)。省略すると既定値。
    params: Option<Value>,
    title: Option<String>,
}

/// `propose_module_rig` の引数。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModuleRigArgs {
    pub(crate) path: String,
    /// `module_rig_output_schema` の形 (ugoe-ai が読む)。省略すると既定値。
    params: Option<Value>,
    /// 組み方 (`ModuleRigOptions`)。省略した欄は既定。
    follow: Option<FollowMode>,
    colliders: Option<bool>,
    poses: Option<bool>,
    title: Option<String>,
}

/// `import_psd` の引数。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImportArgs {
    pub(crate) path: String,
    pub(crate) psd: String,
    name: Option<String>,
    /// 着せ替えのモジュールのプロジェクトを作るときの本体 (`.ugoeproj` か `.ugoe`)。`slot` と一緒に渡す。
    pub(crate) base: Option<String>,
    /// モジュールを付ける本体のスロット。
    slot: Option<String>,
    #[serde(default)]
    pub(crate) discard_changes: bool,
}

/// 自動リギングのツールの定義。
pub(crate) fn tools() -> Vec<Value> {
    let options = ClassifyOptions::default();
    let per_call = options.max_layers_per_request;
    let mut label_items = output_schema()["properties"]["layers"].clone();
    label_items["items"]["properties"]["index"]["description"] =
        json!("The layer number from get_psd_layers.");
    label_items["minItems"] = json!(1);
    let mut variant_sets = output_schema()["properties"]["variant_sets"].clone();
    variant_sets["description"] = json!(
        "Groups of layer numbers that are alternative drawings of the same part, shown one at a \
         time (sleeve A / B, a hat and a cap worn in the same place). Not overlays drawn on top \
         of each other (shadows, line art, highlights) and not open / closed eyes or mouths. \
         Replaces every group set before; pass [] to clear them. In a module project, \
         propose_module_rig turns each group of costume layers into a pose group."
    );
    let mut params = rig_output_schema();
    params["description"] =
        json!("Values the template cannot decide (see get_rig_guide). Omit to use the defaults.");
    let mut module_params = module_rig_output_schema();
    module_params["description"] = json!(
        "Values the template cannot decide (see get_module_rig_guide). Omit to use the defaults."
    );
    vec![
        tool_def(
            "import_psd",
            "Import PSD",
            Kind::Read,
            "PSD を取り込んで新しいプロジェクトを作る (レイヤーごとのメッシュ。save_project まで \
             ディスクには書かない)。続けて get_psd_layers でレイヤーを見て分類する。\
             PSD から作った既存のプロジェクトなら、open_project して get_psd_layers に psd を渡す。\
             base と slot を渡すと、本体のスロットに付ける着せ替えのモジュールのプロジェクトを作る \
             (衣装の PSD を取り付け先の下に取り込む。リグは get_module_rig_guide と propose_module_rig)。",
            object_schema(
                json!({
                    "path": path_property(),
                    "psd": { "type": "string", "description": format!("取り込む PSD のパス。{PATH_NOTE}") },
                    "name": { "type": "string", "description": "プロジェクトの名前 (既定は PSD のファイル名。モジュールではモジュールの名前)" },
                    "base": { "type": "string", "description": format!("モジュールのプロジェクトを作るときの本体 (.ugoeproj か .ugoe) のパス。slot と一緒に渡す。{PATH_NOTE}") },
                    "slot": { "type": "string", "description": "モジュールを付ける本体のスロットの ID (get_model_summary の slots)" },
                    "discard_changes": { "type": "boolean", "description": "同じパスで開いているプロジェクトの未保存の変更を捨てる" },
                }),
                &["path", "psd"],
                None,
            ),
        ),
        tool_def(
            "get_psd_layers",
            "Get PSD layers",
            Kind::Read,
            &format!(
                "PSD のレイヤーの一覧 (番号・名前・キャンバスの画素での外接矩形・対応するメッシュ・\
                 今の分類) と画像を返す。画像は全体の合成画像 (長い辺 {} 画素まで。最初のページだけ) と、\
                 各レイヤーのサムネイル (長い辺 {} 画素まで。薄い灰色の上) で、`layer N: 名前` の文の直後に \
                 そのレイヤーの画像が続く。既定で 1 回に {DEFAULT_COUNT} 枚 (count で {per_call} 枚まで): \
                 next_start / start で続きを、indices で特定のレイヤーを見る。最初のページには分類の指示と語彙 (instructions) が付く。\
                 分類は名前の規則による案 (set_by=rules) が入っているので、画像を見て直すものを \
                 set_layer_labels で渡す。",
                options.overview_max_side, options.thumbnail_max_side
            ),
            object_schema(
                json!({
                    "path": path_property(),
                    "psd": { "type": "string", "description": format!("このモデルの元の PSD のパス (import_psd で作ったプロジェクトでは不要。一度渡せば覚える)。{PATH_NOTE}") },
                    "start": { "type": "integer", "minimum": 0, "description": "最初のレイヤーの番号 (既定 0)" },
                    "count": { "type": "integer", "minimum": 1, "maximum": per_call, "description": format!("返すレイヤーの数 (既定 {DEFAULT_COUNT})") },
                    "indices": {
                        "type": "array",
                        "items": { "type": "integer", "minimum": 0 },
                        "maxItems": per_call,
                        "description": "見るレイヤーの番号 (start / count の代わり)",
                    },
                    "images": { "type": "boolean", "description": "サムネイルを返す (既定 true。false なら一覧だけ)" },
                    "overview": { "type": "boolean", "description": "全体の合成画像を返す (既定は start=0 のときだけ)" },
                    "thumbnail_size": {
                        "type": "integer",
                        "minimum": 32,
                        "maximum": options.thumbnail_max_side,
                        "description": format!("サムネイルの長い辺の上限 (画素。既定 {})", options.thumbnail_max_side),
                    },
                }),
                &["path"],
                None,
            ),
        ),
        tool_def(
            "set_layer_labels",
            "Set layer labels",
            Kind::Read,
            "エージェントが決めたレイヤーの分類を渡す (get_psd_layers の番号・語彙・左右)。渡さなかった \
             レイヤーは今の分類のまま。reset=true なら先に名前の規則の案に戻す (差分の組も消える)。\
             variant_sets には同じ部品の別の絵 (一度に 1 つだけ見せる袖 A / B・同じ所にかぶる帽子と \
             キャップ) のレイヤーの番号の組を渡す (渡すと前の組をすべて置き換える。影・線画のように \
             重ねて描く絵と、目や口の開閉は組にしない)。layers と variant_sets の少なくとも一方を渡す。\
             モデルは変えない。結果はメッシュとの対応・差分の組・確かめたほうがよいレイヤー・リグを組めるか \
             (rig.ready)。左右はキャラクター自身の左右 (正面を向いたキャラクターの左目は画像の右側)。",
            object_schema(
                json!({
                    "path": path_property(),
                    "layers": label_items,
                    "variant_sets": variant_sets,
                    "reset": { "type": "boolean", "description": "先に分類を名前の規則の案に戻す (差分の組も消える)" },
                }),
                &["path"],
                None,
            ),
        ),
        tool_def(
            "get_rig_guide",
            "Get rig guide",
            Kind::Read,
            "リグのテンプレートで決められない値 (振り向きの量・目の閉じ方・口の開き方) の説明と範囲 \
             (instructions)・今の分類から見つけた顔・目・口の位置 (layout)・値の形 (params_schema)・\
             既定値と、全体の画像を返す。画像を見て値を選び、propose_rig に渡す。",
            object_schema(json!({ "path": path_property() }), &["path"], None),
        ),
        tool_def(
            "propose_rig",
            "Propose rig",
            Kind::Read,
            "今の分類とテンプレートでリグ (AngleX/Y/Z・EyeL/ROpen・MouthOpenY) を組む提案を作る。\
             モデルは変えない (複製に適用して検証し、差分とコマンドの要約を返す)。値が範囲の外か、\
             組んだリグが検証を通らなければエラーを返すので、直して呼び直す。成功したら提案の ID を \
             返すので、ユーザーの承認を得てから apply_proposal で適用し、render_png で確かめる。",
            object_schema(
                json!({
                    "path": path_property(),
                    "params": params,
                    "title": { "type": "string", "description": "提案の題 (既定はテンプレートの題)" },
                }),
                &["path"],
                None,
            ),
        ),
        tool_def(
            "get_module_rig_guide",
            "Get module rig guide",
            Kind::Read,
            "着せ替えのモジュールのプロジェクトで、リグのテンプレートで決められない値 (揺れの大きさと \
             振り子の長さ) の説明と範囲 (instructions)・衣装のメッシュごとのラベル・位置・従う本体の \
             ワープ・揺れるか (layout)・値の形 (params_schema)・既定値・組み方の既定 (options)・差分の組 \
             (variant_sets) と、衣装の原画を返す。画像を見て値を選び、propose_module_rig に渡す。",
            object_schema(json!({ "path": path_property() }), &["path"], None),
        ),
        tool_def(
            "propose_module_rig",
            "Propose module rig",
            Kind::Read,
            "着せ替えのモジュールのプロジェクトで、今の分類とテンプレートでリグを組む提案を作る \
             (パラメータを足さず、本体を変えない)。衣装を本体の変形に追従させ、揺れるラベル (髪・尻尾・\
             耳飾り・首の飾り・下の服) を物理で揺らしてスロットが公開する本体の衝突体に当て、衣装の差分 \
             (set_layer_labels の variant_sets の組・名前の末尾の A / B・PSD で片方を隠した重なる絵) を \
             ポーズにする。モデルは変えない。値が範囲の外か、組んだリグが検証を通らなければエラーを返すので、\
             直して呼び直す。成功したら提案の ID を返すので、ユーザーの承認を得てから apply_proposal で適用する。",
            object_schema(
                json!({
                    "path": path_property(),
                    "params": module_params,
                    "follow": {
                        "type": "string",
                        "enum": ["warp", "mesh"],
                        "description": "本体のワープへの追従の形。warp (既定) は衣装を追従のワープで包む (少しずれるが、衣装を描き足しても組み直さずに済む)。mesh は衣装のメッシュのキーフォームに直接写す (ずれないが、頂点を編集したら組み直す)",
                    },
                    "colliders": { "type": "boolean", "description": "揺れをスロットが公開する本体の衝突体に当てる (既定 true)" },
                    "poses": { "type": "boolean", "description": "衣装の差分をポーズにする (既定 true)" },
                    "title": { "type": "string", "description": "提案の題 (既定はテンプレートの題)" },
                }),
                &["path"],
                None,
            ),
        ),
    ]
}

fn ai_error(e: AiError) -> String {
    match e {
        // 「モデルが返した」という前置きは、エージェントが渡した値には合わないので外す。
        AiError::InvalidProposal(m) | AiError::InvalidResponse(m) | AiError::InvalidRequest(m) => m,
        e => e.to_string(),
    }
}

/// PSD から取り込んだ新しいプロジェクト (まだ持ち主に渡していない)。
pub(crate) struct Imported {
    pub(crate) meta: ProjectMeta,
    pub(crate) document: Document,
    pub(crate) source: PsdSource,
    pub(crate) value: Value,
}

/// PSD から新しいプロジェクトを作る (まだ持ち主に開かせない)。
/// `psd` は解決したパス、`base` は `args.base` を解決したパス (`Tools::resolve`)。`base` があれば
/// `args.slot` に付ける着せ替えのモジュールのプロジェクトを作り、エディタと同じく描画順をスロットの範囲に写して
/// 取り付け先の下に取り込む (`docs/editor.md#モジュールのプロジェクト`)。
pub(crate) fn import_psd(
    args: ImportArgs,
    psd: PathBuf,
    base: Option<PathBuf>,
) -> Result<Imported, String> {
    let name = args.name.unwrap_or_else(|| {
        psd.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled".to_owned())
    });
    let project = match (&base, &args.slot) {
        (None, None) => Project::empty(name.clone()),
        (Some(base), Some(slot)) => Project::new_module(&name, base, &SlotId::from(slot.as_str()))
            .map_err(|e| format!("the module project could not be created: {e}"))?,
        _ => return Err("pass `base` and `slot` together to create a module project".into()),
    };
    let source = PsdSource::load(psd.clone())?;
    let mut document = project.document();
    let mut import = layer_tree_command(
        document.model(),
        &source.tree,
        &PsdImportOptions::new(name.clone()),
    )
    .map_err(|e| e.to_string())?;
    if let Some(context) = document.module() {
        context.map_new_meshes(&mut import.command);
    }
    document
        .apply(import.command)
        .map_err(|e| format!("the PSD could not be imported: {e}"))?;
    let module = document
        .module()
        .map(|context| json!({ "base": context.base().path, "slot": context.slot().id }));
    let next = if module.is_some() {
        "get_psd_layers でレイヤーと画像を見て分類し、get_module_rig_guide → propose_module_rig"
    } else {
        "get_psd_layers でレイヤーと画像を見て分類する"
    };
    let value = json!({
        "name": name,
        "psd": psd,
        "layers": source.labels.len(),
        // モジュールなら衣装のメッシュだけを数える。
        "meshes": document
            .model()
            .meshes
            .iter()
            .filter(|m| document.module().is_none_or(|c| !c.is_base_mesh(&m.id)))
            .count(),
        "module": module,
        "warnings": import.warnings,
        "next": next,
    });
    Ok(Imported {
        meta: project.meta,
        document,
        source,
        value,
    })
}

/// 覚えている PSD。`psd` (解決したパス) を渡されたら (違う PSD なら) 読み直し、分類を名前の規則の案に戻す。
fn source(slot: &mut Option<PsdSource>, psd: Option<PathBuf>) -> Result<&mut PsdSource, String> {
    if let Some(path) = psd
        && slot.as_ref().is_none_or(|s| s.path != path)
    {
        *slot = Some(PsdSource::load(path)?);
    }
    slot.as_mut().ok_or_else(|| NO_PSD.to_owned())
}

/// `doc` はプロジェクトの文書 (の複製)、`slot` は覚えている PSD、`psd` は `args.psd` を解決したパス
/// (`Tools::resolve`)。
pub(crate) fn get_psd_layers(
    doc: &Document,
    slot: &mut Option<PsdSource>,
    args: LayersArgs,
    psd: Option<PathBuf>,
) -> ToolResult {
    let defaults = ClassifyOptions::default();
    let per_call = defaults.max_layers_per_request;
    let meshes = unique_mesh_names(doc);
    let source = source(slot, psd)?;
    let infos = layers(&source.tree);
    let total = infos.len();
    let selected: Vec<usize> = match &args.indices {
        Some(indices) => {
            if indices.len() > per_call {
                return Err(format!("at most {per_call} layers per call"));
            }
            if let Some(i) = indices.iter().find(|&&i| i >= total) {
                return Err(format!(
                    "layer {i} does not exist (there are {total} layers, numbered from 0)"
                ));
            }
            indices.clone()
        }
        None => {
            if args.start >= total && total > 0 {
                return Err(format!(
                    "start {} is past the last layer (there are {total} layers, numbered from 0)",
                    args.start
                ));
            }
            let count = args.count.unwrap_or(DEFAULT_COUNT).clamp(1, per_call);
            (args.start..(args.start + count).min(total)).collect()
        }
    };
    let size = args
        .thumbnail_size
        .unwrap_or(defaults.thumbnail_max_side)
        .clamp(32, defaults.thumbnail_max_side);
    let options = ClassifyOptions {
        thumbnail_max_side: size,
        thumbnail_min_side: defaults.thumbnail_min_side.min(size),
        ..defaults
    };
    let first_page = args.indices.is_none() && args.start == 0;
    let with_overview = args.overview.unwrap_or(first_page);

    let mut content = Vec::new();
    if with_overview {
        content.push(Content::Text(format!(
            "overview: the whole illustration (visible layers), scaled so the long side is at \
             most {} px; the canvas is {}x{} px",
            options.overview_max_side, source.tree.width, source.tree.height
        )));
        content.push(Content::Png(
            overview_png(&source.tree, options.overview_max_side).map_err(ai_error)?,
        ));
    }
    let mut listed = Vec::with_capacity(selected.len());
    for &i in &selected {
        let info = &infos[i];
        let label = &source.labels[i];
        if args.images && info.has_pixels {
            let png = layer_thumbnail_png(&source.tree, i, &options).map_err(ai_error)?;
            if let Some(png) = png {
                content.push(Content::Text(format!(
                    "layer {i}: {}",
                    info.layer_path.join(" / ")
                )));
                content.push(Content::Png(png));
            }
        }
        let name = info.layer_path.last().map(String::as_str).unwrap_or("");
        listed.push(json!({
            "index": i,
            "path": info.layer_path,
            "bbox": info.bbox,
            "visible": info.visible,
            "has_pixels": info.has_pixels,
            "mesh": meshes.get(name),
            "label": label.label,
            "side": label.side,
            "confidence": (f64::from(label.confidence) * 100.0).round() / 100.0,
            "set_by": if source.by_agent.contains(&i) { "agent" } else { "rules" },
            "needs_review": label.needs_review(&options),
            "variant": label.variant,
        }));
    }
    let next_start = match (&args.indices, selected.last()) {
        (None, Some(&last)) if last + 1 < total => Some(last + 1),
        _ => None,
    };
    let mut value = json!({
        "psd": source.path,
        "canvas": { "width": source.tree.width, "height": source.tree.height },
        "total": total,
        "layers": listed,
        "next_start": next_start,
        "images": if args.images {
            "After this JSON: the overview (if requested), then for each listed layer with pixels \
             a text `layer N: path` followed by its thumbnail, cropped to the layer's own bounds \
             and drawn over light gray. bbox is x, y, width, height in canvas pixels."
        } else {
            "No thumbnails (images=false). bbox is x, y, width, height in canvas pixels."
        },
    });
    if first_page {
        value["instructions"] = json!(classification_instructions());
    }
    Ok(Output { value, content })
}

/// 名前が 1 つのメッシュだけのもの: 名前 → ID (`mesh_parts` と同じく名前で対応付ける)。
fn unique_mesh_names(doc: &Document) -> BTreeMap<String, String> {
    let mut by_name: BTreeMap<String, Option<String>> = BTreeMap::new();
    for mesh in &doc.model().meshes {
        by_name
            .entry(mesh.name.clone())
            .and_modify(|id| *id = None)
            .or_insert_with(|| Some(mesh.id.as_str().to_owned()));
    }
    by_name
        .into_iter()
        .filter_map(|(name, id)| Some((name, id?)))
        .collect()
}

pub(crate) fn set_layer_labels(
    doc: &Document,
    slot: &mut Option<PsdSource>,
    args: LabelsArgs,
) -> ToolResult {
    if args.layers.is_none() && args.variant_sets.is_none() {
        return Err("pass `layers`, `variant_sets` or both".into());
    }
    let source = source(slot, None)?;
    let mut labels = if args.reset {
        classify_by_name(&source.tree)
    } else {
        source.labels.clone()
    };
    let layers = args.layers.unwrap_or_else(|| json!([]));
    let indices: Vec<usize> = layers
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| l.get("index")?.as_u64())
        .filter_map(|i| usize::try_from(i).ok())
        .collect();
    let mut answer = json!({ "layers": layers });
    if let Some(sets) = args.variant_sets {
        answer["variant_sets"] = sets;
    }
    let updated = apply_answer(&mut labels, answer).map_err(ai_error)?;
    source.labels = labels;
    if args.reset {
        source.by_agent.clear();
    }
    source.by_agent.extend(indices);
    let mut value = labels_summary(doc, source);
    value["updated"] = json!(updated);
    Ok(value.into())
}

/// 分類の要約: メッシュとの対応・対応しないレイヤー・確かめたほうがよいレイヤー・リグを組めるか。
fn labels_summary(doc: &Document, source: &PsdSource) -> Value {
    let model = doc.model();
    let parts = mesh_parts(model, &source.labels);
    let options = ClassifyOptions::default();
    let meshes: Vec<Value> = model
        .meshes
        .iter()
        .filter_map(|m| {
            let part = parts.get(&m.id)?;
            Some(json!({ "mesh": m.id, "name": m.name, "label": part.label, "side": part.side }))
        })
        .collect();
    let matched: BTreeSet<&str> = model
        .meshes
        .iter()
        .filter(|m| parts.contains_key(&m.id))
        .map(|m| m.name.as_str())
        .collect();
    let mut unmatched = Vec::new();
    let mut review = Vec::new();
    for (i, label) in source.labels.iter().enumerate() {
        let name = label.layer_path.last().map(String::as_str).unwrap_or("");
        if label.label != Label::Other && !matched.contains(name) {
            unmatched.push(json!({ "index": i, "path": label.layer_path, "label": label.label }));
        }
        if label.needs_review(&options) {
            review.push(i);
        }
    }
    // モジュールのプロジェクトでは、モジュールのリグを組めるか (衣装のメッシュが取り付け先の下にあるか)。
    let ready = if doc.module().is_some() {
        let parts = module_mesh_parts(doc, &source.labels);
        module_rig_request_guided(doc, &parts, &[], &[], &ModuleRigGuidance::default()).map(|_| ())
    } else {
        rig_request(model, &parts, &[], &[]).map(|_| ())
    };
    let tool = if doc.module().is_some() {
        "propose_module_rig"
    } else {
        "propose_rig"
    };
    let rig = match ready {
        Ok(()) => json!({ "ready": true, "tool": tool }),
        Err(e) => json!({ "ready": false, "tool": tool, "error": ai_error(e) }),
    };
    json!({
        "layers": source.labels.len(),
        "set_by_agent": source.by_agent.len(),
        "meshes": meshes,
        "variant_sets": variant_sets(&source.labels),
        "unmatched_layers": unmatched,
        "needs_review": review,
        "rig": rig,
    })
}

/// 差分の組 (レイヤーの番号の組。組の番号の順)。
fn variant_sets(labels: &[LayerLabel]) -> Vec<Vec<usize>> {
    let mut sets: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (i, label) in labels.iter().enumerate() {
        if let Some(id) = label.variant {
            sets.entry(id).or_default().push(i);
        }
    }
    sets.into_values().collect()
}

pub(crate) fn get_rig_guide(doc: &Document, slot: &Option<PsdSource>) -> ToolResult {
    if doc.module().is_some() {
        return Err(USE_MODULE_RIG.into());
    }
    let source = slot.as_ref().ok_or(NO_PSD)?;
    let model = doc.model();
    let parts = mesh_parts(model, &source.labels);
    let max_side = ClassifyOptions::default().overview_max_side;
    let image = overview_png(&source.tree, max_side).map_err(ai_error)?;
    // エディタが API に送るリクエストと同じ文 (システムプロンプトが値の説明、プロンプトが位置)。
    let request = rig_request(model, &parts, &image, &[]).map_err(ai_error)?;
    Ok(Output {
        value: json!({
            "instructions": request.system,
            "layout": request.prompt,
            "params_schema": rig_output_schema(),
            "defaults": RigParams::default(),
            "labels": { "layers": source.labels.len(), "set_by_agent": source.by_agent.len() },
            "next": "画像を見て値を選び、propose_rig の params に渡す",
        }),
        content: vec![
            Content::Text(format!(
                "the whole illustration (visible layers), long side at most {max_side} px"
            )),
            Content::Png(image),
        ],
    })
}

/// リグの提案を作る (置くのは呼び出し側)。
pub(crate) fn propose_rig(
    doc: &Document,
    slot: &Option<PsdSource>,
    args: RigArgs,
) -> Result<Draft, String> {
    if doc.module().is_some() {
        return Err(USE_MODULE_RIG.into());
    }
    let source = slot.as_ref().ok_or(NO_PSD)?;
    let model = doc.model();
    let (params, reason) = match &args.params {
        Some(answer) => {
            let (params, reason) = params_from_answer(answer).map_err(ai_error)?;
            (params, Some(reason))
        }
        None => (RigParams::default(), None),
    };
    let parts = mesh_parts(model, &source.labels);
    let proposal = template_rig(model, &parts, &params, &RigOptions::default()).map_err(|e| {
        format!(
            "the rig could not be built: {}; fix it and call propose_rig again",
            ai_error(e)
        )
    })?;
    let values = format!(
        "turn {:.2}, eye close line {:.2}, bend {:+.2}, mouth open {:.2}",
        params.turn_angle_scale,
        params.eye_close_line,
        params.eye_close_bend,
        params.mouth_open_height
    );
    let head = match &reason {
        Some(reason) => format!("Values chosen by the agent: {values} ({reason})"),
        None => format!("Default values: {values}"),
    };
    let by_agent = source.by_agent.len();
    let labels = format!(
        "Labels: the agent chose {by_agent} of {total} layers; the rest follow the name rules",
        total = source.labels.len()
    );
    let title = args.title.unwrap_or_else(|| {
        format!(
            "{} — {}",
            proposal.title,
            if reason.is_some() {
                "agent"
            } else {
                "defaults"
            }
        )
    });
    Ok(Draft {
        title,
        rationale: format!("{head}\n{labels}\n{}", proposal.rationale),
        commands: proposal.commands,
        extra: json!({ "params": params }),
    })
}

/// モジュールのリグのツールの入力: モジュールのプロジェクトであることと、覚えている PSD を確かめる。
fn module_source<'a>(doc: &Document, slot: &'a Option<PsdSource>) -> Result<&'a PsdSource, String> {
    if doc.module().is_none() {
        return Err(NOT_A_MODULE.into());
    }
    slot.as_ref().ok_or_else(|| NO_PSD.to_owned())
}

pub(crate) fn get_module_rig_guide(doc: &Document, slot: &Option<PsdSource>) -> ToolResult {
    let source = module_source(doc, slot)?;
    let parts = module_mesh_parts(doc, &source.labels);
    let max_side = ClassifyOptions::default().overview_max_side;
    // エディタが API に送るものと同じく、衣装の PSD の原画 (`docs/ai-integration.md#モジュールの自動リギング`)。
    let image = overview_png(&source.tree, max_side).map_err(ai_error)?;
    let request =
        module_rig_request_guided(doc, &parts, &image, &[], &ModuleRigGuidance::default())
            .map_err(ai_error)?;
    let context = doc.module().ok_or(NOT_A_MODULE)?;
    Ok(Output {
        value: json!({
            "instructions": request.system,
            "layout": request.prompt,
            "params_schema": module_rig_output_schema(),
            "defaults": ModuleRigParams::default(),
            "options": ModuleRigOptions::default(),
            "slot": context.slot().id,
            "variant_sets": variant_sets(&source.labels),
            "labels": { "layers": source.labels.len(), "set_by_agent": source.by_agent.len() },
            "next": "画像を見て値を選び、propose_module_rig の params に渡す",
        }),
        content: vec![
            Content::Text(format!(
                "the costume drawing (visible layers), long side at most {max_side} px"
            )),
            Content::Png(image),
        ],
    })
}

/// モジュールのリグの提案を作る (置くのは呼び出し側)。
pub(crate) fn propose_module_rig(
    doc: &Document,
    slot: &Option<PsdSource>,
    args: ModuleRigArgs,
) -> Result<Draft, String> {
    let source = module_source(doc, slot)?;
    let (params, reason) = match &args.params {
        Some(answer) => {
            let (params, reason) = module_params_from_answer(answer).map_err(ai_error)?;
            (params, Some(reason))
        }
        None => (ModuleRigParams::default(), None),
    };
    let defaults = ModuleRigOptions::default();
    let options = ModuleRigOptions {
        follow: args.follow.unwrap_or(defaults.follow),
        colliders: args.colliders.unwrap_or(defaults.colliders),
        poses: args.poses.unwrap_or(defaults.poses),
    };
    let parts = module_mesh_parts(doc, &source.labels);
    let proposal = template_module_rig(doc, &parts, &params, &options).map_err(|e| {
        format!(
            "the module rig could not be built: {}; fix it and call propose_module_rig again",
            ai_error(e)
        )
    })?;
    let values = format!(
        "sway {:.2}, pendulum length {:.2}",
        params.sway_scale, params.sway_length
    );
    let head = match &reason {
        Some(reason) => format!("Values chosen by the agent: {values} ({reason})"),
        None => format!("Default values: {values}"),
    };
    let labels = format!(
        "Labels: the agent chose {} of {} layers and {} variant set(s); the rest follow the name rules",
        source.by_agent.len(),
        source.labels.len(),
        variant_sets(&source.labels).len()
    );
    let title = args.title.unwrap_or_else(|| {
        format!(
            "{} — {}",
            proposal.title,
            if reason.is_some() {
                "agent"
            } else {
                "defaults"
            }
        )
    });
    Ok(Draft {
        title,
        rationale: format!("{head}\n{labels}\n{}", proposal.rationale),
        commands: proposal.commands,
        extra: json!({ "params": params, "options": options }),
    })
}
