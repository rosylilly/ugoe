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
//! エディタがプロジェクトに残した分類の答え (`.ugoe-cache/classification.json`。`ugoe_editor_core::classification_cache`) は、
//! 同じ PSD (識別が合うもの) を読むときに名前の規則の案の代わりに使い、PSD を渡されていなければそのキャッシュの PSD を読む
//! (書くのはエディタだけ。`docs/ai-integration.md#エージェントに任せる自動リギング`)。
//!
//! エージェントの分類はサーバのメモリにだけ持つ。`propose_rig`・`propose_module_rig` は、エージェントが決めたレイヤーの分類を
//! 分類の直し (`Command::SetLayerCorrection`) としてリグの提案の先頭に添え、利用者が承認すると `project.json` に残る (T0570)。
//!
//! 提案の適用は `apply_proposal` (利用者の確認を通す。ADR 0006)。画像の大きさと 1 回に返す枚数は、
//! API で分類するときと同じ上限 (`ClassifyOptions` の既定値) に合わせる。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};
use ugoe_ai::AiError;
use ugoe_ai::classify::{
    ClassifyOptions, Label, LayerLabel, apply_answer, cached_labels, classification_instructions,
    classify_by_name, corrected_labels, correction_label, label_correction, layer_thumbnail_png,
    layers, mark_name_disagreements, output_schema, overview_png,
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
use ugoe_editor_core::classification_cache::{CachedClassification, read_classification_cache};
use ugoe_editor_core::import::{layer_tree_command, read_layer_tree};
use ugoe_editor_core::message::MessageArgs;
use ugoe_editor_core::{
    Coded, Command, Document, Project, ProjectMeta, PsdImportOptions, message_args,
};
use ugoe_fs::read_regular_file;
use ugoe_model::SlotId;
use ugoe_psd::{ImportLimits, LayerTree, MAX_PSD_FILE_BYTES};

use crate::schema::object_schema;
use crate::tools::{
    Content, Draft, Kind, Output, PATH_NOTE, ToolError, ToolResult, path_property, tool_def,
};

/// PSD がまだ分からないときの断りの文 (`mcp.no_psd`)。
const NO_PSD: &str = "no PSD is associated with this project: call import_psd, or pass `psd` \
                      (the PSD this model was imported from) to get_psd_layers";

/// モジュールのプロジェクトで本体向けのリグのツールを呼んだときの断りの文 (`mcp.module_project`)。
const USE_MODULE_RIG: &str = "this is a module project, which cannot add parameters: use \
                              get_module_rig_guide and propose_module_rig instead";

/// モジュールのプロジェクトでないときにモジュールのリグのツールを呼んだときの断りの文 (`mcp.not_a_module`)。
const NOT_A_MODULE: &str = "this is not a module project: use get_rig_guide and propose_rig, or \
                            create a module project with import_psd and `base` and `slot`";

/// `propose_rig`・`propose_module_rig` の `keep_labels` の説明。
const KEEP_LABELS: &str = "エージェントが set_layer_labels で決めた分類 (利用者が直していないレイヤー) を、分類の直し \
                           (set_layer_correction) として提案の先頭に添える (既定 true)。承認されるとプロジェクトに残り、\
                           開き直しても set_by=user で戻る。外すコマンドの番号は結果の label_commands";

/// `get_psd_layers` が既定で 1 回に返すレイヤーの数。上限 (`ClassifyOptions::max_layers_per_request`) より
/// 小さくする: MCP クライアントは画像を含む結果の大きさに上限を設けることがある (Claude Code は既定で
/// 25,000 トークン。画像も数える) ので、既定では小分けにし、多く返すのは `count` で頼まれたときだけにする。
const DEFAULT_COUNT: usize = 8;

/// 自動リギングの元にする PSD と、レイヤーの分類。
///
/// 利用者がエディタで直した分類 (`project.json` の `editor.classification`。文書の
/// [`Document::layer_corrections`]) は、ここには入れず、使うたびに上に重ねる ([`PsdSource::labels_for`])。
/// 直しは文書の状態 (Undo の対象) なので、重ねる時の文書に従う。
pub(crate) struct PsdSource {
    path: PathBuf,
    tree: LayerTree,
    /// 分類 (`classify_by_name` と同じ並び)。エージェントが決めていないレイヤーは、エディタが残した AI の分類
    /// ([`PsdSource::saved`]) か名前の規則の案。利用者の直しは含まない。
    labels: Vec<LayerLabel>,
    /// エージェントが決めたレイヤーの番号。
    by_agent: BTreeSet<usize>,
    /// 分類がエディタの残した AI の分類 (`.ugoe-cache/classification.json`) から始まったか。
    saved: bool,
}

impl fmt::Debug for PsdSource {
    // 画素を書き出さない。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PsdSource")
            .field("path", &self.path)
            .field("layers", &self.labels.len())
            .field("by_agent", &self.by_agent.len())
            .field("saved", &self.saved)
            .finish()
    }
}

impl PsdSource {
    fn load(path: PathBuf) -> Result<Self, ToolError> {
        Self::load_with(path, MAX_PSD_FILE_BYTES, ImportLimits::default())
    }

    /// PSD を読み、プロジェクトのディレクトリ `project` にエディタが残した分類の答えが同じ PSD (識別が合う) の
    /// ものなら、名前の規則の案の代わりにそれを使う。キャッシュが読めない・合わないときは名前の規則の案。
    fn load_for(path: PathBuf, project: &Path) -> Result<Self, ToolError> {
        let (mut source, bytes) =
            Self::read_with(path, MAX_PSD_FILE_BYTES, ImportLimits::default())?;
        if let Ok(Some(cache)) = read_classification_cache::<LayerLabel>(project) {
            source.restore(cache, &bytes);
        }
        Ok(source)
    }

    /// エディタが残した分類 `cache` が、読んだ PSD (`bytes`) と合えば使う。
    fn restore(&mut self, cache: CachedClassification<LayerLabel>, bytes: &[u8]) {
        if !cache.identity.matches(bytes) {
            return;
        }
        if let Some(labels) = cached_labels(&self.tree, cache.layers) {
            self.labels = labels;
            self.saved = cache.by_ai;
        }
    }

    /// PSD を、ファイルの大きさの上限 `max_bytes` と展開の上限 `limits` で読む。読めなければ、エラーのコード
    /// (`psd.total_limit_exceeded` など。`docs/editor.md#エラーと警告のコード`) と引数 ([`ToolError::coded`]) を
    /// 添えて返す (どの上限を超えたかをエージェントが `structuredContent` で見分けられるように)。
    fn load_with(path: PathBuf, max_bytes: u64, limits: ImportLimits) -> Result<Self, ToolError> {
        Self::read_with(path, max_bytes, limits).map(|(source, _)| source)
    }

    /// [`PsdSource::load_with`] と同じだが、読んだ PSD のバイト列も返す (分類のキャッシュと比べるため)。
    fn read_with(
        path: PathBuf,
        max_bytes: u64,
        limits: ImportLimits,
    ) -> Result<(Self, Vec<u8>), ToolError> {
        // 装置 (`/dev/zero`)・FIFO・ディレクトリは開かず、上限より大きいファイルは読まない (終わらない読み込みで
        // メモリを使い果たさないように)。開いた後にも通常のファイルかを確かめる (`ugoe_fs::read_regular_file`)。
        // コードはエディタの PSD の読み込みと同じ `psd.read_file_failed`。
        let bytes = read_regular_file(&path, max_bytes).map_err(|e| {
            let shown = path.display().to_string();
            ToolError::mcp(
                "psd.read_file_failed",
                message_args! { "path" => shown.as_str(), "detail" => e.to_string() },
                format!("{shown}: {e}"),
            )
        })?;
        let tree = read_layer_tree(&bytes, limits)
            .map_err(|e| ToolError::coded(format!("{}: {e}", path.display()), e.to_message()))?;
        let source = Self {
            labels: classify_by_name(&tree),
            by_agent: BTreeSet::new(),
            saved: false,
            path,
            tree,
        };
        Ok((source, bytes))
    }
}

/// 利用者の直しを重ねた分類 ([`PsdSource::labels_for`])。
struct Labels {
    /// 分類 (`classify_by_name` と同じ並び)。直したレイヤーは直しのラベル・左右で、確からしさ 1。
    labels: Vec<LayerLabel>,
    /// 利用者が直したレイヤーの番号。
    corrected: BTreeSet<usize>,
    /// エージェントが決めたレイヤーのうち、利用者が直していないものの数。
    by_agent: usize,
    /// 残りのレイヤーがエディタの残した AI の分類 ([`PsdSource::saved`]) か (偽なら名前の規則)。
    saved: bool,
}

impl PsdSource {
    /// 分類に文書 `doc` の利用者の直しを、エディタと同じ規則で重ねる (`ugoe_ai::classify::corrected_labels`。
    /// `docs/editor.md#分類の直し`): 直しはレイヤーのパスで対応させ、直したレイヤーは確からしさ 1・名前の規則との
    /// 食い違いの印なし。語彙にないラベルの直しは重ねない。
    fn labels_for(&self, doc: &Document) -> Labels {
        let corrections = doc.layer_corrections();
        let corrected: BTreeSet<usize> = self
            .labels
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                corrections
                    .get(&l.layer_path)
                    .and_then(correction_label)
                    .is_some()
            })
            .map(|(i, _)| i)
            .collect();
        Labels {
            labels: corrected_labels(&self.labels, corrections),
            by_agent: self.by_agent.difference(&corrected).count(),
            saved: self.saved,
            corrected,
        }
    }
}

/// レイヤー `i` の分類を決めたもの (利用者の直し・エージェント・エディタが残した AI の分類・名前の規則)。
fn set_by(source: &PsdSource, labels: &Labels, i: usize) -> &'static str {
    if labels.corrected.contains(&i) {
        "user"
    } else if source.by_agent.contains(&i) {
        "agent"
    } else if source.saved {
        "editor"
    } else {
        "rules"
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
    /// エージェントが決めた分類を、分類の直しとして提案に添える (既定 true)。
    keep_labels: Option<bool>,
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
    /// エージェントが決めた分類を、分類の直しとして提案に添える (既定 true)。
    keep_labels: Option<bool>,
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
                 set_layer_labels で渡す。name_disagreement は、渡した分類が名前の規則の確からしい答えと \
                 食い違うときの名前の規則のラベル (label) か左右 (side) で、needs_review になる \
                 (画像で確かめ、名前が正しければ直す)。set_by=user は利用者がエディタで直したレイヤー \
                 (プロジェクトの editor.classification) で、確からしさ 1。set_layer_labels でも変わらない。\
                 set_by=editor はエディタが AI で分類してプロジェクトに残した答え (.ugoe-cache/classification.json。\
                 同じ PSD のときだけ使う。psd を渡さなければその PSD を読む)。",
                options.overview_max_side, options.thumbnail_max_side
            ),
            object_schema(
                json!({
                    "path": path_property(),
                    "psd": { "type": "string", "description": format!("このモデルの元の PSD のパス (import_psd で作ったプロジェクトと、エディタで分類したプロジェクトでは不要。一度渡せば覚える)。{PATH_NOTE}") },
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
             モデルは変えない。結果はメッシュとの対応・差分の組・確かめたほうがよいレイヤー・名前の規則の \
             確からしい答えと食い違うレイヤー (name_disagreements。左右の取り違えは目の開閉などを入れ替える) \
             ・リグを組めるか (rig.ready)。左右はキャラクター自身の左右 (正面を向いたキャラクターの左目は画像の右側)。\
             利用者がエディタで直したレイヤー (get_psd_layers の set_by=user) は上書きしない (答えは覚えるが \
             直しが優先し、その番号を kept_corrections に返す。reset でも直しは消えない)。\
             答えはサーバのメモリにだけ持つ。propose_rig・propose_module_rig が分類の直しとして提案に添え、\
             利用者が承認するとプロジェクトに残る (リグなしで残す・直すなら propose_edit で set_layer_correction)。",
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
             返すので、ユーザーの承認を得てから apply_proposal で適用し、render_png で確かめる。\
             エージェントが決めた分類は分類の直しとして提案の先頭に添える (keep_labels)。",
            object_schema(
                json!({
                    "path": path_property(),
                    "params": params,
                    "title": { "type": "string", "description": "提案の題 (既定はテンプレートの題)" },
                    "keep_labels": { "type": "boolean", "description": KEEP_LABELS },
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
             直して呼び直す。成功したら提案の ID を返すので、ユーザーの承認を得てから apply_proposal で適用する。\
             エージェントが決めた分類は分類の直しとして提案の先頭に添える (keep_labels)。",
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
                    "keep_labels": { "type": "boolean", "description": KEEP_LABELS },
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

/// 自動リギングの処理の失敗を、コード `code` (`mcp.*`) のツールのエラーにする。
fn ai(code: &'static str) -> impl Fn(AiError) -> ToolError {
    move |e| ToolError::mcp(code, MessageArgs::new(), ai_error(e))
}

fn no_psd() -> ToolError {
    ToolError::mcp("mcp.no_psd", MessageArgs::new(), NO_PSD)
}

fn use_module_rig() -> ToolError {
    ToolError::mcp("mcp.module_project", MessageArgs::new(), USE_MODULE_RIG)
}

fn not_a_module() -> ToolError {
    ToolError::mcp("mcp.not_a_module", MessageArgs::new(), NOT_A_MODULE)
}

fn rig_failed(message: String) -> ToolError {
    ToolError::mcp("mcp.rig_failed", MessageArgs::new(), message)
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
) -> Result<Imported, ToolError> {
    let name = args.name.unwrap_or_else(|| {
        psd.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled".to_owned())
    });
    let project = match (&base, &args.slot) {
        (None, None) => Project::empty(name.clone()),
        (Some(base), Some(slot)) => Project::new_module(&name, base, &SlotId::from(slot.as_str()))
            .map_err(|e| {
                ToolError::coded(
                    format!("the module project could not be created: {e}"),
                    e.to_message(),
                )
            })?,
        _ => {
            return Err(ToolError::invalid_arguments(
                "pass `base` and `slot` together to create a module project",
            ));
        }
    };
    let source = PsdSource::load(psd.clone())?;
    let mut document = project.document();
    let mut import = layer_tree_command(
        document.model(),
        &source.tree,
        &PsdImportOptions::new(name.clone()),
    )
    .map_err(|e| ToolError::coded(e.to_string(), e.to_message()))?;
    if let Some(context) = document.module() {
        context.map_new_meshes(&mut import.command);
    }
    document.apply(import.command).map_err(|e| {
        ToolError::coded(
            format!("the PSD could not be imported: {e}"),
            e.to_message(),
        )
    })?;
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

/// 覚えている PSD。`psd` (解決したパス) を渡されたら (違う PSD なら) 読み直し、分類を名前の規則の案
/// (プロジェクト `project` にエディタが残した同じ PSD の分類があればそれ) に戻す。渡されず、覚えてもいなければ、
/// エディタが残した分類の PSD を読む ([`recall`])。
fn source<'a>(
    slot: &'a mut Option<PsdSource>,
    psd: Option<PathBuf>,
    project: &Path,
    resolve: &dyn Fn(&Path) -> Result<PathBuf, ToolError>,
) -> Result<&'a mut PsdSource, ToolError> {
    match psd {
        Some(path) => {
            if slot.as_ref().is_none_or(|s| s.path != path) {
                *slot = Some(PsdSource::load_for(path, project)?);
            }
        }
        None => recall(slot, project, resolve),
    }
    slot.as_mut().ok_or_else(no_psd)
}

/// PSD を覚えていなければ、プロジェクトのディレクトリ `project` にエディタが残した分類 (`.ugoe-cache/classification.json`)
/// の PSD を読んで覚える (パスは `resolve` で確かめる。`--root` の外なら読まない)。キャッシュがない・読めない・PSD が
/// 変わったときは何もしない (覚えていないまま。ツールは `mcp.no_psd` を返す)。
pub(crate) fn recall(
    slot: &mut Option<PsdSource>,
    project: &Path,
    resolve: &dyn Fn(&Path) -> Result<PathBuf, ToolError>,
) {
    if slot.is_some() {
        return;
    }
    let Ok(Some(cache)) = read_classification_cache::<LayerLabel>(project) else {
        return;
    };
    let Ok(path) = resolve(&cache.psd) else {
        return;
    };
    let Ok((mut source, bytes)) =
        PsdSource::read_with(path, MAX_PSD_FILE_BYTES, ImportLimits::default())
    else {
        return;
    };
    if !cache.identity.matches(&bytes) {
        return;
    }
    source.restore(cache, &bytes);
    *slot = Some(source);
}

/// `doc` はプロジェクトの文書 (の複製)、`slot` は覚えている PSD、`psd` は `args.psd` を解決したパス
/// (`Tools::resolve`)。
pub(crate) fn get_psd_layers(
    doc: &Document,
    slot: &mut Option<PsdSource>,
    args: LayersArgs,
    psd: Option<PathBuf>,
    project: &Path,
    resolve: &dyn Fn(&Path) -> Result<PathBuf, ToolError>,
) -> ToolResult {
    let defaults = ClassifyOptions::default();
    let per_call = defaults.max_layers_per_request;
    let meshes = unique_mesh_names(doc);
    let source = source(slot, psd, project, resolve)?;
    let effective = source.labels_for(doc);
    let infos = layers(&source.tree);
    let total = infos.len();
    let selected: Vec<usize> = match &args.indices {
        Some(indices) => {
            if indices.len() > per_call {
                return Err(ToolError::mcp(
                    "mcp.too_many_layers",
                    message_args! { "max" => per_call },
                    format!("at most {per_call} layers per call"),
                ));
            }
            if let Some(&i) = indices.iter().find(|&&i| i >= total) {
                return Err(ToolError::mcp(
                    "mcp.layer_out_of_range",
                    message_args! { "index" => i, "count" => total },
                    format!("layer {i} does not exist (there are {total} layers, numbered from 0)"),
                ));
            }
            indices.clone()
        }
        None => {
            if args.start >= total && total > 0 {
                return Err(ToolError::mcp(
                    "mcp.layer_out_of_range",
                    message_args! { "index" => args.start, "count" => total },
                    format!(
                        "start {} is past the last layer (there are {total} layers, numbered from 0)",
                        args.start
                    ),
                ));
            }
            let count = args.count.unwrap_or(DEFAULT_COUNT).clamp(1, per_call);
            (args.start..args.start.saturating_add(count).min(total)).collect()
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
            overview_png(&source.tree, options.overview_max_side)
                .map_err(ai("mcp.image_failed"))?,
        ));
    }
    let mut listed = Vec::with_capacity(selected.len());
    for &i in &selected {
        let info = &infos[i];
        let label = &effective.labels[i];
        if args.images && info.has_pixels {
            let png =
                layer_thumbnail_png(&source.tree, i, &options).map_err(ai("mcp.image_failed"))?;
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
            "set_by": set_by(source, &effective, i),
            "needs_review": label.needs_review(&options),
            "name_disagreement": label.name_disagreement,
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
    project: &Path,
    resolve: &dyn Fn(&Path) -> Result<PathBuf, ToolError>,
) -> ToolResult {
    if args.layers.is_none() && args.variant_sets.is_none() {
        return Err(ToolError::invalid_arguments(
            "pass `layers`, `variant_sets` or both",
        ));
    }
    let source = source(slot, None, project, resolve)?;
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
    let updated = apply_answer(&mut labels, answer).map_err(ai("mcp.invalid_labels"))?;
    mark_name_disagreements(&source.tree, &mut labels).map_err(ai("mcp.invalid_labels"))?;
    source.labels = labels;
    if args.reset {
        source.by_agent.clear();
        source.saved = false;
    }
    source.by_agent.extend(indices.iter().copied());
    let effective = source.labels_for(doc);
    // 利用者が直したレイヤーへの答えは、覚えるが直しの下に置く (直しを消せば効く)。
    let kept: BTreeSet<usize> = indices
        .iter()
        .copied()
        .filter(|i| effective.corrected.contains(i))
        .collect();
    let mut value = labels_summary(doc, &effective);
    value["updated"] = json!(updated);
    value["kept_corrections"] = json!(kept);
    Ok(value.into())
}

/// 分類の要約: メッシュとの対応・対応しないレイヤー・確かめたほうがよいレイヤー・名前の規則と食い違うレイヤー・
/// リグを組めるか。
fn labels_summary(doc: &Document, effective: &Labels) -> Value {
    let model = doc.model();
    let labels = &effective.labels;
    let parts = mesh_parts(model, labels);
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
    let mut disagreements = Vec::new();
    for (i, label) in labels.iter().enumerate() {
        let name = label.layer_path.last().map(String::as_str).unwrap_or("");
        if label.label != Label::Other && !matched.contains(name) {
            unmatched.push(json!({ "index": i, "path": label.layer_path, "label": label.label }));
        }
        if label.needs_review(&options) {
            review.push(i);
        }
        if let Some(by_name) = label.name_disagreement {
            disagreements.push(json!({
                "index": i,
                "path": label.layer_path,
                "label": label.label,
                "side": label.side,
                "by_name": by_name,
            }));
        }
    }
    // モジュールのプロジェクトでは、モジュールのリグを組めるか (衣装のメッシュが取り付け先の下にあるか)。
    let ready = if doc.module().is_some() {
        let parts = module_mesh_parts(doc, labels);
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
        "layers": labels.len(),
        "set_by_agent": effective.by_agent,
        "corrected_by_user": effective.corrected,
        "saved_by_editor": effective.saved,
        "meshes": meshes,
        "variant_sets": variant_sets(labels),
        "unmatched_layers": unmatched,
        "needs_review": review,
        "name_disagreements": disagreements,
        "rig": rig,
    })
}

/// リグのガイドに添える分類の数 (レイヤー・エージェントが決めた・利用者が直した)。
fn labels_count(effective: &Labels) -> Value {
    json!({
        "layers": effective.labels.len(),
        "set_by_agent": effective.by_agent,
        "corrected_by_user": effective.corrected.len(),
        "saved_by_editor": effective.saved,
    })
}

/// 提案の理由に書く、分類を決めたものの内訳。
fn labels_note(effective: &Labels) -> String {
    let total = effective.labels.len();
    let by_agent = effective.by_agent;
    let rest = if effective.saved {
        "the classification the editor saved"
    } else {
        "the name rules"
    };
    match effective.corrected.len() {
        0 => format!("the agent chose {by_agent} of {total} layers; the rest follow {rest}"),
        corrected => format!(
            "the user corrected {corrected} and the agent chose {by_agent} of {total} layers; \
             the rest follow {rest}"
        ),
    }
}

/// リグの提案に添える分類の直し: エージェントが決めたレイヤーのうち、利用者が直していないものの今の分類 (T0570)。
/// 利用者が承認すると `project.json` の `editor.classification` に残り、開き直しても戻る。`keep` が偽なら添えない。
/// 直したレイヤーはリグが直しで組まれているので、エージェントの答えで上書きしない。
fn kept_labels(source: &PsdSource, effective: &Labels, keep: Option<bool>) -> Vec<Command> {
    if !keep.unwrap_or(true) {
        return Vec::new();
    }
    source
        .by_agent
        .iter()
        .filter(|i| !effective.corrected.contains(i))
        .filter_map(|&i| effective.labels.get(i))
        .map(|label| Command::SetLayerCorrection {
            path: label.layer_path.clone(),
            correction: Some(label_correction(label)),
        })
        .collect()
}

/// 提案の理由に書く、添えた分類の直しの数。
fn kept_note(kept: usize) -> String {
    match kept {
        0 => String::new(),
        n => format!(
            "; commands 0-{} keep the agent's labels of {n} layer(s) in the project \
             (exclude them to adopt only the rig)",
            n - 1
        ),
    }
}

/// 添えた分類の直しのコマンドの番号 (提案の先頭から)。
fn label_commands(kept: usize) -> Vec<usize> {
    (0..kept).collect()
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
        return Err(use_module_rig());
    }
    let source = slot.as_ref().ok_or_else(no_psd)?;
    let effective = source.labels_for(doc);
    let model = doc.model();
    let parts = mesh_parts(model, &effective.labels);
    let max_side = ClassifyOptions::default().overview_max_side;
    let image = overview_png(&source.tree, max_side).map_err(ai("mcp.image_failed"))?;
    // エディタが API に送るリクエストと同じ文 (システムプロンプトが値の説明、プロンプトが位置)。
    let request = rig_request(model, &parts, &image, &[]).map_err(ai("mcp.rig_not_ready"))?;
    Ok(Output {
        value: json!({
            "instructions": request.system,
            "layout": request.prompt,
            "params_schema": rig_output_schema(),
            "defaults": RigParams::default(),
            "labels": labels_count(&effective),
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
) -> Result<Draft, ToolError> {
    if doc.module().is_some() {
        return Err(use_module_rig());
    }
    let source = slot.as_ref().ok_or_else(no_psd)?;
    let model = doc.model();
    let (params, reason) = match &args.params {
        Some(answer) => {
            let (params, reason) =
                params_from_answer(answer).map_err(ai("mcp.invalid_rig_params"))?;
            (params, Some(reason))
        }
        None => (RigParams::default(), None),
    };
    let effective = source.labels_for(doc);
    let parts = mesh_parts(model, &effective.labels);
    let kept = kept_labels(source, &effective, args.keep_labels);
    let proposal = template_rig(doc, &parts, &params, &RigOptions::default()).map_err(|e| {
        rig_failed(format!(
            "the rig could not be built: {}; fix it and call propose_rig again",
            ai_error(e)
        ))
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
    let labels = format!(
        "Labels: {}{}",
        labels_note(&effective),
        kept_note(kept.len())
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
        extra: json!({ "params": params, "label_commands": label_commands(kept.len()) }),
        commands: kept.into_iter().chain(proposal.commands).collect(),
    })
}

/// モジュールのリグのツールの入力: モジュールのプロジェクトであることと、覚えている PSD を確かめる。
fn module_source<'a>(
    doc: &Document,
    slot: &'a Option<PsdSource>,
) -> Result<&'a PsdSource, ToolError> {
    if doc.module().is_none() {
        return Err(not_a_module());
    }
    slot.as_ref().ok_or_else(no_psd)
}

pub(crate) fn get_module_rig_guide(doc: &Document, slot: &Option<PsdSource>) -> ToolResult {
    let source = module_source(doc, slot)?;
    let effective = source.labels_for(doc);
    let parts = module_mesh_parts(doc, &effective.labels);
    let max_side = ClassifyOptions::default().overview_max_side;
    // エディタが API に送るものと同じく、衣装の PSD の原画 (`docs/ai-integration.md#モジュールの自動リギング`)。
    let image = overview_png(&source.tree, max_side).map_err(ai("mcp.image_failed"))?;
    let request =
        module_rig_request_guided(doc, &parts, &image, &[], &ModuleRigGuidance::default())
            .map_err(ai("mcp.rig_not_ready"))?;
    let context = doc.module().ok_or_else(not_a_module)?;
    Ok(Output {
        value: json!({
            "instructions": request.system,
            "layout": request.prompt,
            "params_schema": module_rig_output_schema(),
            "defaults": ModuleRigParams::default(),
            "options": ModuleRigOptions::default(),
            "slot": context.slot().id,
            "variant_sets": variant_sets(&effective.labels),
            "labels": labels_count(&effective),
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
) -> Result<Draft, ToolError> {
    let source = module_source(doc, slot)?;
    let (params, reason) = match &args.params {
        Some(answer) => {
            let (params, reason) =
                module_params_from_answer(answer).map_err(ai("mcp.invalid_rig_params"))?;
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
    let effective = source.labels_for(doc);
    let parts = module_mesh_parts(doc, &effective.labels);
    let kept = kept_labels(source, &effective, args.keep_labels);
    let proposal = template_module_rig(doc, &parts, &params, &options).map_err(|e| {
        rig_failed(format!(
            "the module rig could not be built: {}; fix it and call propose_module_rig again",
            ai_error(e)
        ))
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
        "Labels: {}; {} variant set(s){}",
        labels_note(&effective),
        variant_sets(&effective.labels).len(),
        kept_note(kept.len())
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
        extra: json!({
            "params": params,
            "options": options,
            "label_commands": label_commands(kept.len()),
        }),
        commands: kept.into_iter().chain(proposal.commands).collect(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// 通常のファイルでないもの (`/dev/zero` のような終わらない装置・FIFO) は読まない
    /// (読み続けてメモリを使い果たし、エディタごと落ちないように)。
    #[cfg(unix)]
    #[test]
    fn refuses_to_read_a_psd_that_is_not_a_regular_file() {
        let err = PsdSource::load(PathBuf::from("/dev/null")).unwrap_err();
        assert!(err.message().contains("not a regular file"), "{err}");
        let err = PsdSource::load(std::env::temp_dir()).unwrap_err();
        assert!(err.message().contains("not a regular file"), "{err}");
        // FIFO は開くだけで書き手を待って止まるので、開かずに拒む。
        let dir = tempfile::tempdir().unwrap();
        let fifo = dir.path().join("x.psd");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .is_ok_and(|s| s.success());
        if made {
            let err = PsdSource::load(fifo).unwrap_err();
            assert!(err.message().contains("not a regular file"), "{err}");
        }
    }

    /// 上限より大きい PSD は読まない (ファイルの大きさで拒み、展開まで進まない)。
    #[test]
    fn refuses_to_read_a_psd_larger_than_the_limit() {
        let psd =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fixtures/layers.psd");
        let len = std::fs::metadata(&psd).unwrap().len();
        let err = PsdSource::load_with(psd.clone(), len - 1, ImportLimits::default()).unwrap_err();
        assert!(err.message().contains("file is too large"), "{err}");
        assert!(
            err.message()
                .contains(&format!("the limit is {} bytes", len - 1)),
            "{err}"
        );
        PsdSource::load_with(psd, len, ImportLimits::default()).unwrap();
    }

    #[test]
    fn exceeding_an_import_limit_names_the_limit() {
        let psd =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fixtures/layers.psd");
        let limits = ImportLimits::default().with_max_channel_decompressed(100);
        let err = PsdSource::load_with(psd.clone(), MAX_PSD_FILE_BYTES, limits).unwrap_err();
        assert!(
            err.message().ends_with(
                "a PSD channel decompresses to 3072 bytes, more than the import limit of 100 bytes"
            ),
            "{err}"
        );
        // 文の末尾にコードは付けない (コードは `structuredContent` で返す)。
        assert!(!err.message().contains("(code:"), "{err}");
        assert_eq!(err.detail().code, "psd.channel_limit_exceeded");
        let limits = ImportLimits::default().with_max_total_decompressed(5000);
        let err = PsdSource::load_with(psd.clone(), MAX_PSD_FILE_BYTES, limits).unwrap_err();
        assert!(!err.message().contains("(code:"), "{err}");
        // コードと引数は構造で返す (`structuredContent`)。
        let detail = err.detail();
        assert_eq!(detail.code, "psd.total_limit_exceeded");
        assert_eq!(detail.args["max"], 5000u32.into());
        assert_eq!(detail.message, err.message());
        PsdSource::load(psd).unwrap();
    }

    #[test]
    fn a_start_past_an_empty_psd_does_not_overflow() {
        let tree = LayerTree {
            width: 1,
            height: 1,
            root: Vec::new(),
            warnings: Vec::new(),
        };
        let mut slot = Some(PsdSource {
            path: PathBuf::from("empty.psd"),
            labels: classify_by_name(&tree),
            by_agent: BTreeSet::new(),
            saved: false,
            tree,
        });
        let args: LayersArgs =
            serde_json::from_value(json!({ "path": "x", "start": usize::MAX, "images": false }))
                .unwrap();
        let page = get_psd_layers(
            &Document::new(ugoe_model::fixtures::single_quad()),
            &mut slot,
            args,
            None,
            Path::new("."),
            &|p: &Path| Ok(p.to_path_buf()),
        )
        .unwrap();
        assert_eq!(page.value["layers"], json!([]));
        assert_eq!(page.value["total"], 0);
    }
}
