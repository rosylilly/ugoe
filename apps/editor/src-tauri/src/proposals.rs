//! AI の提案 (PSD のレイヤーの分類 → 自動リギング) と、提案のプレビュー・採用・却下の IPC コマンド。
//! 方針: ADR 0006・`docs/editor.md#ai-の提案`。提案の置き場・プレビュー・採用の規則は
//! `ugoe_editor_core::proposal` にあり、ここは入力 (PSD・分類) を集めて ugoe-ai を呼び、結果を渡すだけ。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。
//!
//! - 外部に送るもの (`use_ai`) は必ず [`crate::ai::prepare_requests`] を通し、同意がなければ送らずに
//!   `AiReply::NeedsConsent` を返す。AI を使わない経路 (名前の規則とテンプレートの既定値) は何も送らない。
//! - 提案は採用するまでドキュメントを変えない。プレビューは複製に適用したもの ([`ProposalSession`] が持つ)。
//! - モジュールのプロジェクト (ADR 0024) では、リグの提案は `ugoe_ai::module_rig` (パラメータを足さず、本体の変形に
//!   追従させ、揺れる衣装に物理を付ける) を使う。分類・送る前の確認・作り直しの流れはモデルと同じ
//!   (`docs/editor.md#モジュールのプロジェクトでの-ai-の提案`)。
//! - 利用者の注文 (`instructions`) は AI を使うときだけ送る。作り直し ([`regenerate_rig_proposal`]) は前の答え
//!   (値と理由) と注文を送って新しい提案を作り、前の提案の後ろに並べる (置き換えない)。注文はプロジェクトに
//!   保存しない (`docs/editor.md#ai-への注文と作り直し`)。
//! - 法線マップの提案 ([`propose_normal_maps`]) は AI を使わず、何も送らない (`ugoe_ai::normal`。まとまりと設定の
//!   決め方は `crate::lighting`)。コマンドをまとまりごとの区切り ([`CommandSection`]) にして画面に渡す
//!   (`docs/editor.md#法線マップの提案`)。
//! - モジュールのリグの提案は、衣装のメッシュのキーフォームに本体の変形を頂点ごとに写すことがある。採用した後に
//!   そのメッシュの形 (頂点・三角形・親) を編集したら、写した変形が合わなくなるので、提案し直すよう知らせる
//!   ([`ProposalState::stale_rigs`])。採用したときの形の指紋は `project.json` の `editor.rigged_meshes` に残し
//!   (保存するまでファイルには書かない)、開き直しても知らせる (`docs/editor.md#モジュールのプロジェクトでの-ai-の提案`)。
//! - 分類の答えは、プロジェクトの `.ugoe-cache/classification.json` に分類した PSD の識別と一緒に残し
//!   (`ugoe_editor_core::classification_cache`)、開いたときに PSD が合えば戻す。分類したとき (保存先があれば) と
//!   保存したときに書く。キャッシュなので Undo の対象にせず、`project.json` も変えない
//!   (`docs/editor.md#分類の答えのキャッシュ`)。

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use ts_rs::TS;
use ugoe_ai::classify::{
    ClassifyGuidance, ClassifyOptions, Label, LayerLabel, cached_labels,
    classification_requests_guided, classify_by_name, classify_layers_guided, corrected_labels,
    correction_label, overview_png,
};
use ugoe_ai::instructions::Instructions;
use ugoe_ai::module_rig::{
    FollowMode, ModuleRigGuidance, ModuleRigOptions, ModuleRigParams, PreviousModuleRig,
    module_mesh_parts, module_rig_request_guided, propose_module_rig,
    propose_module_rig_with_ai_guided,
};
use ugoe_ai::rig::{
    DEFAULT_MAX_ATTEMPTS, PreviousRig, RigGuidance, RigOptions, RigParams, mesh_parts, propose_rig,
    propose_rig_with_ai_guided, rig_request_guided,
};
use ugoe_ai::{AiError, Proposal, Provider, ProviderRequest};
use ugoe_editor_core::classification_cache::{
    CachedClassification, PsdIdentity, read_classification_cache, write_classification_cache,
};
use ugoe_editor_core::{
    ApplyReport, Command, Document, KeyformTarget, LayerCorrections, Message, ProposalOrigin,
    ProposalView, Proposals, PsdImportError, import,
};
use ugoe_model::{Mesh, MeshId};
use ugoe_psd::{ImportLimits, LayerTree, MAX_PSD_FILE_BYTES};

use crate::AppState;
use crate::ai::{AiReply, ConsentAnswer, Prepared, lock_project, prepare_instructed};
use crate::ai_provider::Purpose;
use crate::commands::blocking;
use crate::error::{EditorError, IpcResult};
use crate::lighting::{CommandSection, NormalMapGroupChoice, normal_map_proposal_in};
use crate::project::{ProjectSession, absolute_path};
use ugoe_fs::read_regular_file;

/// 開いているプロジェクトの AI の提案と、その入力 (`ProjectSession` が持つ。プロジェクトを替えると消える)。
#[derive(Debug, Default)]
pub struct ProposalSession {
    /// 提案の元にする PSD (「New from PSD…」「Import PSD…」で取り込んだもの、または分類で選んだもの)。
    psd: Option<PathBuf>,
    classification: Option<Classified>,
    proposals: Proposals,
    /// 最後にプレビューした提案を適用した複製 (`get_proposal_preview_bundle` で渡す)。
    preview: Option<Document>,
    /// このパネルで作ったリグの提案の値と理由 (提案の番号ごと。作り直しで前の答えとして送る)。
    rigs: HashMap<u32, PreviousAnswer>,
    /// 提案のコマンドの区切り (提案の番号ごと。法線マップの提案のまとまり)。
    sections: HashMap<u32, Vec<CommandSection>>,
    /// モジュールのリグの提案がキーフォームを書く衣装のメッシュと、そのコマンドの番号 (提案の番号ごと)。
    writes: HashMap<u32, Vec<(usize, MeshId)>>,
    /// 採用したモジュールのリグの提案がキーフォームを書いた衣装のメッシュ (ファイルの中の ID。[`file_id`]) と、
    /// 採用したときの形の指紋 ([`shape_digest`])。`project.json` の `editor.rigged_meshes` に残す。
    rigged: BTreeMap<String, String>,
}

/// `project.json` の `editor` のうち、本体の変形を写した衣装のメッシュの形の指紋を置く項目
/// (`{"<メッシュの ID>": "<16 桁の 16 進>"}`。ID はファイルの中の ID)。
pub(crate) const RIGGED_MESHES_KEY: &str = "rigged_meshes";

/// メッシュや親のデフォーマの、ファイルの中の ID (モジュールのプロジェクトでは、作業用のモデルの接頭辞を外したもの。
/// 名前が同じなら開き直しても変わらない)。
fn file_id<'a>(document: &Document, id: &'a str) -> &'a str {
    match document.module() {
        Some(context) => context.display_id(id),
        None => id,
    }
}

/// 本体の変形を写したメッシュの形 (親・UV・三角形。これが変わったら、写した変形は合わない) の指紋。
///
/// ファイルに残すので、Rust の版で変わりうる `DefaultHasher` ではなく FNV-1a (64 ビット) で作る。
fn shape_digest(document: &Document, mesh: &Mesh) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    let mut feed = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(PRIME);
        }
    };
    match &mesh.parent {
        Some(parent) => {
            feed(&[1]);
            let id = file_id(document, parent.as_str());
            feed(&(id.len() as u64).to_le_bytes());
            feed(id.as_bytes());
        }
        None => feed(&[0]),
    }
    feed(&(mesh.uvs.len() as u64).to_le_bytes());
    for uv in &mesh.uvs {
        feed(&uv.x.to_bits().to_le_bytes());
        feed(&uv.y.to_bits().to_le_bytes());
    }
    feed(&(mesh.indices.len() as u64).to_le_bytes());
    for index in &mesh.indices {
        feed(&index.to_le_bytes());
    }
    format!("{hash:016x}")
}

/// 提案のコマンドのうち、メッシュのキーフォームを書くもの (番号とメッシュ)。
fn mesh_writes(commands: &[Command]) -> Vec<(usize, MeshId)> {
    commands
        .iter()
        .enumerate()
        .filter_map(|(i, command)| match command {
            Command::AddKeyformKey {
                target: KeyformTarget::Mesh(id),
                ..
            }
            | Command::SetKeyformCell {
                target: KeyformTarget::Mesh(id),
                ..
            } => Some((i, id.clone())),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone)]
struct Classified {
    psd: PathBuf,
    /// 分類したときの PSD の識別 (キャッシュに残し、開いたときに今の PSD と比べる)。
    identity: PsdIdentity,
    labels: Vec<LayerLabel>,
    by_ai: bool,
    /// AI で分類したときに添えた注文。
    instructions: Option<Instructions>,
}

impl ProposalSession {
    /// 提案の元にする PSD を覚える。前と違う PSD なら、前の分類を捨てる。
    pub(crate) fn set_psd(&mut self, path: PathBuf) {
        if self.classification.as_ref().is_some_and(|c| c.psd != path) {
            self.classification = None;
        }
        self.psd = Some(path);
    }

    /// 提案の元にする PSD (読めるパスの 1 つ。`crate::access`)。
    pub(crate) fn psd(&self) -> Option<&Path> {
        self.psd.as_deref()
    }

    /// 最後に分類した PSD のレイヤーのラベルに、`document` の分類の直しを重ねたもの (法線マップのまとまりに使う。
    /// `crate::lighting`)。
    pub(crate) fn layer_labels(&self, document: &Document) -> Option<Vec<LayerLabel>> {
        self.classification
            .as_ref()
            .map(|c| corrected_labels(&c.labels, document.layer_corrections()))
    }

    /// 最後の分類に `document` の分類の直しを重ねたもの。
    fn corrected(&self, document: &Document) -> Option<Classified> {
        self.classification.as_ref().map(|c| Classified {
            labels: corrected_labels(&c.labels, document.layer_corrections()),
            ..c.clone()
        })
    }

    /// 最後の分類を、キャッシュ (`.ugoe-cache/classification.json`) に書く形にしたもの。直しは重ねない
    /// (直しは `project.json` に残る)。分類していなければ `None`。
    pub(crate) fn cache(&self) -> Option<CachedClassification<LayerLabel>> {
        self.classification.as_ref().map(|c| CachedClassification {
            psd: c.psd.clone(),
            identity: c.identity,
            by_ai: c.by_ai,
            instructions: c.instructions.as_ref().map(|i| i.as_str().to_owned()),
            layers: c.labels.clone(),
        })
    }

    /// プロジェクトのディレクトリ `dir` のキャッシュから分類を戻す (開いたとき)。キャッシュの PSD が今もあり、
    /// 識別とレイヤーが合うときだけ戻し、その PSD を提案の元にする。合わない・読めないときは何もしない。
    pub(crate) fn restore_classification(&mut self, dir: &Path) {
        match restored_classification(dir) {
            Ok(Some(classified)) => {
                self.psd = Some(classified.psd.clone());
                self.classification = Some(classified);
            }
            Ok(None) => {}
            Err(e) => eprintln!("could not restore the layer classification: {e}"),
        }
    }

    /// 提案の置き場 (エディタにつないだエージェントの提案もここに置く。`crate::mcp`)。
    pub(crate) fn proposals_mut(&mut self) -> &mut Proposals {
        &mut self.proposals
    }

    /// 採用したモジュールのリグの提案がキーフォームを書いた衣装のメッシュのうち、その後に形を編集したもの
    /// (メッシュの順)。今キーフォームの軸を持たないもの (採用を Undo したもの) と、消えたものは除く。
    pub(crate) fn stale_rigs(&self, document: &Document) -> Vec<StaleRig> {
        if self.rigged.is_empty() {
            return Vec::new();
        }
        document
            .model()
            .meshes
            .iter()
            .filter(|mesh| {
                !mesh.keyforms.bindings.is_empty()
                    && self
                        .rigged
                        .get(file_id(document, mesh.id.as_str()))
                        .is_some_and(|digest| *digest != shape_digest(document, mesh))
            })
            .map(|mesh| StaleRig {
                mesh: mesh.id.to_string(),
                name: mesh.name.clone(),
            })
            .collect()
    }

    /// 採用した提案 `id` (`excluded` を除く) がキーフォームを書いたメッシュの、今の形を覚える。
    fn remember_rigged(&mut self, document: &Document, id: u32, excluded: &[usize]) {
        let Some(writes) = self.writes.get(&id) else {
            return;
        };
        for (_, mesh) in writes.iter().filter(|(i, _)| !excluded.contains(i)) {
            if let Some(found) = document.model().meshes.iter().find(|m| &m.id == mesh) {
                self.rigged.insert(
                    file_id(document, mesh.as_str()).to_owned(),
                    shape_digest(document, found),
                );
            }
        }
    }

    /// 覚えている形を、今の形で覚え直す (知らせを消す)。
    fn forget_stale(&mut self, document: &Document) {
        for mesh in &document.model().meshes {
            if let Some(digest) = self.rigged.get_mut(file_id(document, mesh.id.as_str())) {
                *digest = shape_digest(document, mesh);
            }
        }
    }

    /// `project.json` の `editor.rigged_meshes` から、覚えた形の指紋を読む (開いたとき)。形の違う値は捨てる。
    pub(crate) fn restore_rigged(&mut self, value: Option<&serde_json::Value>) {
        self.rigged = value
            .and_then(serde_json::Value::as_object)
            .map(|map| {
                map.iter()
                    .filter_map(|(id, digest)| Some((id.clone(), digest.as_str()?.to_owned())))
                    .collect()
            })
            .unwrap_or_default();
    }

    /// `editor.rigged_meshes` に書く値 (覚えたものがなければ `None` で、項目を書かない)。
    pub(crate) fn rigged_value(&self) -> Option<serde_json::Value> {
        (!self.rigged.is_empty()).then(|| {
            serde_json::Value::Object(
                self.rigged
                    .iter()
                    .map(|(id, digest)| (id.clone(), digest.clone().into()))
                    .collect(),
            )
        })
    }

    fn state(&self, document: &Document) -> ProposalState {
        let proposals = self.proposals.views(document);
        ProposalState {
            stale_rigs: self.stale_rigs(document),
            regenerable: proposals
                .iter()
                .map(|p| p.id)
                .filter(|id| self.rigs.contains_key(id))
                .collect(),
            sections: proposals
                .iter()
                .filter_map(|p| {
                    self.sections.get(&p.id).map(|sections| ProposalSections {
                        id: p.id,
                        sections: sections.clone(),
                    })
                })
                .collect(),
            proposals,
            psd: self.psd.as_ref().map(|p| p.display().to_string()),
            classification: self
                .classification
                .as_ref()
                .map(|c| c.view(document.layer_corrections())),
        }
    }
}

// ---- 画面に渡す型 ----

/// 1 枚のレイヤーの分類 (`ugoe_ai::classify::LayerLabel`)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct LayerLabelView {
    /// レイヤーの名前の列 (外側のグループから順に、最後がレイヤー自身)。
    pub path: Vec<String>,
    /// 意味ラベル (`face`・`eye_white` など。`docs/ai-integration.md#レイヤーの意味分類`)。
    pub label: String,
    /// `left`・`right`・`center` (キャラクター自身の左右)。
    pub side: String,
    pub confidence: f32,
    /// 確からしさが低いか名前の規則と食い違い、人間が確かめたほうがよい。
    pub needs_review: bool,
    /// AI が同じ部品の別の絵 (差分) とみなしたレイヤーの組の番号 (`LayerLabel::variant`)。なければ `null`。
    pub variant: Option<u32>,
    /// AI の分類が名前の規則の確からしい答えと食い違うときの、名前の規則の答え
    /// (`LayerLabel::name_disagreement`)。なければ `null`。
    pub name_disagreement: Option<NameDisagreementView>,
    /// 利用者が直した (または「これでよい」と認めた) か (`docs/editor.md#分類の直し`)。直したレイヤーは
    /// `label`・`side` が直した値で、確からしさは 1、食い違いの印はない。
    pub corrected: bool,
}

/// 名前の規則と AI の分類の食い違い (`ugoe_ai::classify::NameDisagreement`)。食い違う項目にだけ名前の規則の答えが入る。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct NameDisagreementView {
    /// 名前の規則のラベル。ラベルが同じなら `null`。
    pub label: Option<String>,
    /// 名前の規則の左右 (`left`・`right`・`center`)。左右が同じなら `null`。
    pub side: Option<String>,
}

/// PSD のレイヤーの分類の結果。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct Classification {
    /// 分類した PSD のパス。
    pub psd: String,
    /// AI で分類したか (偽なら名前の規則だけ)。
    pub by_ai: bool,
    /// AI で分類したときに添えた注文。
    pub instructions: Option<String>,
    pub layers: Vec<LayerLabelView>,
    /// 直すときに選べるラベル (`ugoe_ai::classify::Label` の語彙。`docs/ai-integration.md#レイヤーの意味分類` の順)。
    pub label_choices: Vec<String>,
}

impl Classified {
    /// 画面に渡す形。`corrections` の直しを重ねる。
    fn view(&self, corrections: &LayerCorrections) -> Classification {
        let options = ClassifyOptions::default();
        let labels = corrected_labels(&self.labels, corrections);
        Classification {
            psd: self.psd.display().to_string(),
            by_ai: self.by_ai,
            instructions: self.instructions.as_ref().map(|i| i.as_str().to_owned()),
            layers: labels
                .iter()
                .map(|l| LayerLabelView {
                    path: l.layer_path.clone(),
                    label: l.label.as_str().to_owned(),
                    side: l.side.as_str().to_owned(),
                    confidence: l.confidence,
                    needs_review: l.needs_review(&options),
                    variant: l.variant,
                    name_disagreement: l.name_disagreement.map(|d| NameDisagreementView {
                        label: d.label.map(|l| l.as_str().to_owned()),
                        side: d.side.map(|s| s.as_str().to_owned()),
                    }),
                    corrected: corrections
                        .get(&l.layer_path)
                        .and_then(correction_label)
                        .is_some(),
                })
                .collect(),
            label_choices: Label::ALL.iter().map(|l| l.as_str().to_owned()).collect(),
        }
    }
}

/// 提案のパネルの状態。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ProposalState {
    /// 採用を待っている提案 (古い順)。
    pub proposals: Vec<ProposalView>,
    /// 提案の元にする PSD のパス。なければ `null` (分類するときに選ぶ)。
    pub psd: Option<String>,
    /// 最後の分類の結果。
    pub classification: Option<Classification>,
    /// 注文を添えて作り直せる提案の番号 (このパネルで作ったリグの提案。つないだエージェントの提案は含まない)。
    pub regenerable: Vec<u32>,
    /// コマンドを区切って出す提案 (法線マップの提案)。ない提案は、同じ対象への続いたコマンドをまとめて出す。
    pub sections: Vec<ProposalSections>,
    /// 採用したモジュールのリグの提案が本体の変形を写した衣装のメッシュのうち、その後に形 (頂点・三角形・親) を
    /// 編集したもの。写した変形が合わないので、提案し直すよう知らせる。
    pub stale_rigs: Vec<StaleRig>,
}

/// 本体の変形を写した後に形を編集した衣装のメッシュ。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct StaleRig {
    pub mesh: String,
    pub name: String,
}

/// 提案 1 つのコマンドの区切り。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ProposalSections {
    pub id: u32,
    pub sections: Vec<CommandSection>,
}

/// 提案を複製に適用した姿 (`preview_proposal` の戻り値)。
#[derive(Debug, Serialize, TS)]
pub struct ProposalPreview {
    /// 適用した後のモデルの JSON (`get_model_json` と同じ形)。テクスチャ付きのバンドルは
    /// `get_proposal_preview_bundle` で受け取る。
    pub model_json: String,
    pub report: ApplyReport,
}

// ---- 分類と提案 (IPC から切り離した本体。テストでプロバイダを差し替える) ----

fn read_tree(path: &Path) -> IpcResult<LayerTree> {
    read_tree_with(path, ImportLimits::default())
}

/// PSD を読み、レイヤーの木と PSD の識別 (分類のキャッシュに残す) を返す。
fn read_psd(path: &Path) -> IpcResult<(LayerTree, PsdIdentity)> {
    read_psd_with(path, ImportLimits::default())
}

/// PSD を展開の上限 `limits` で読む。上限の超過は、どの上限かが分かるコード (`psd.channel_limit_exceeded`・
/// `psd.total_limit_exceeded`) のまま返し、ほかの読み込みのエラーはパス付きの `psd.read_file_failed` にする。
fn read_tree_with(path: &Path, limits: ImportLimits) -> IpcResult<LayerTree> {
    read_psd_with(path, limits).map(|(tree, _)| tree)
}

/// [`read_tree_with`] と同じだが、PSD の識別も返す。
fn read_psd_with(path: &Path, limits: ImportLimits) -> IpcResult<(LayerTree, PsdIdentity)> {
    let bytes =
        read_regular_file(path, MAX_PSD_FILE_BYTES).map_err(|e| EditorError::io(path, e))?;
    let tree = import::read_layer_tree(&bytes, limits).map_err(|e| -> Message {
        match e {
            PsdImportError::Psd(error) => EditorError::psd_read(path, error).into(),
            e => e.into(),
        }
    })?;
    Ok((tree, PsdIdentity::of(&bytes)))
}

/// プロジェクトのディレクトリ `dir` のキャッシュの分類 ([`ProposalSession::restore_classification`])。
/// キャッシュがない・PSD がない・PSD の大きさや中身が違う・レイヤーが合わないときは `None`。
fn restored_classification(dir: &Path) -> IpcResult<Option<Classified>> {
    let Some(cache) = read_classification_cache::<LayerLabel>(dir)? else {
        return Ok(None);
    };
    // 大きさが違えば (消えていれば) 読まない。
    if std::fs::metadata(&cache.psd).map(|m| m.len()).ok() != Some(cache.identity.size) {
        return Ok(None);
    }
    let (tree, identity) = read_psd(&cache.psd)?;
    if identity != cache.identity {
        return Ok(None);
    }
    let Some(labels) = cached_labels(&tree, cache.layers) else {
        return Ok(None);
    };
    Ok(Some(Classified {
        psd: cache.psd,
        identity,
        labels,
        by_ai: cache.by_ai,
        // 今の上限を超える注文は捨てる (分類は戻す)。
        instructions: cache
            .instructions
            .and_then(|text| Instructions::new(&text).ok().flatten()),
    }))
}

/// 分類の答えをプロジェクトのディレクトリ `dir` のキャッシュに書く。書けなくても分類や保存は失敗させない。
pub(crate) fn write_cache(dir: &Path, cache: &CachedClassification<LayerLabel>) {
    if let Err(e) = write_classification_cache(dir, cache) {
        eprintln!("could not write the layer classification cache: {e}");
    }
}

/// レイヤーを分類する。`provider` があれば AI で (注文と前の分類を添えて)、なければ名前の規則で。
pub(crate) fn classify(
    tree: &LayerTree,
    provider: Option<&dyn Provider>,
    guidance: &ClassifyGuidance,
) -> Result<Vec<LayerLabel>, AiError> {
    match provider {
        Some(p) => classify_layers_guided(p, tree, &ClassifyOptions::default(), guidance),
        None => Ok(classify_by_name(tree)),
    }
}

/// 画面から受け取った注文 (空なら `None`、長すぎればエラー)。
fn instructions(text: Option<&str>) -> IpcResult<Option<Instructions>> {
    Instructions::new(text.unwrap_or_default()).map_err(|e| EditorError::Ai(e).into())
}

/// 送る前の確認に示す注文の文字数。
fn instruction_chars(instructions: &Option<Instructions>) -> usize {
    instructions.as_ref().map_or(0, Instructions::chars)
}

/// 原画の PNG (リギングの値を決めるために AI に送る)。
fn overview(tree: &LayerTree) -> Result<Vec<u8>, AiError> {
    overview_png(tree, ClassifyOptions::default().overview_max_side)
}

/// 作り直しで AI に送る前の答え。モデルとモジュールのプロジェクトで AI が決める値が違う。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PreviousAnswer {
    Model(PreviousRig),
    Module(PreviousModuleRig),
}

/// モジュールのプロジェクトのリグの組み方 (AI タブで選ぶ。`ugoe_ai::module_rig::ModuleRigOptions`)。
/// AI には送らない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ModuleRigChoice {
    /// 本体のワープの変形を衣装のメッシュのキーフォームに直接写す (偽なら衣装を追従のワープで包む)。
    pub direct: bool,
    /// 揺れのリグを、スロットが公開する本体の衝突体に当てる。
    pub colliders: bool,
    /// 名前の末尾が A / B … で違うだけの衣装の差分をポーズにする。
    pub poses: bool,
}

impl Default for ModuleRigChoice {
    fn default() -> Self {
        Self::from(ModuleRigOptions::default())
    }
}

impl From<ModuleRigOptions> for ModuleRigChoice {
    fn from(options: ModuleRigOptions) -> Self {
        Self {
            direct: options.follow == FollowMode::Mesh,
            colliders: options.colliders,
            poses: options.poses,
        }
    }
}

impl From<ModuleRigChoice> for ModuleRigOptions {
    fn from(choice: ModuleRigChoice) -> Self {
        Self {
            follow: if choice.direct {
                FollowMode::Mesh
            } else {
                FollowMode::Warp
            },
            colliders: choice.colliders,
            poses: choice.poses,
        }
    }
}

/// リグの提案に添える、利用者の注文と前の答え (作り直し)。
#[derive(Debug, Clone, Default)]
pub(crate) struct RigAsk {
    pub instructions: Option<Instructions>,
    pub previous: Option<PreviousAnswer>,
    /// 同じ注文で AI がレイヤーを分類した (ラベルに関わる注文は分類で反映した)。
    pub labels_follow_instructions: bool,
    /// モジュールのプロジェクトのリグの組み方 (モデルのプロジェクトでは使わない)。
    pub module: ModuleRigChoice,
}

/// 前の答えがモデルとモジュールで食い違う (プロジェクトの種類が変わった) ときの断り。
const OTHER_KIND: &str =
    "the proposal was made for another kind of project, so it cannot be regenerated";

impl RigAsk {
    fn model(&self) -> Result<RigGuidance, AiError> {
        let previous = match &self.previous {
            None => None,
            Some(PreviousAnswer::Model(p)) => Some(p.clone()),
            Some(PreviousAnswer::Module(_)) => {
                return Err(AiError::InvalidRequest(OTHER_KIND.into()));
            }
        };
        Ok(RigGuidance {
            instructions: self.instructions.clone(),
            previous,
            labels_follow_instructions: self.labels_follow_instructions,
        })
    }

    fn module(&self) -> Result<ModuleRigGuidance, AiError> {
        let previous = match &self.previous {
            None => None,
            Some(PreviousAnswer::Module(p)) => Some(p.clone()),
            Some(PreviousAnswer::Model(_)) => {
                return Err(AiError::InvalidRequest(OTHER_KIND.into()));
            }
        };
        Ok(ModuleRigGuidance {
            instructions: self.instructions.clone(),
            previous,
            labels_follow_instructions: self.labels_follow_instructions,
        })
    }
}

/// [`rig`] の結果: 提案と、作り直しに使う値と理由、反映できなかった注文。
#[derive(Debug)]
pub(crate) struct Rigged {
    pub proposal: Proposal,
    pub previous: PreviousAnswer,
    pub unapplied: Vec<String>,
}

/// AI を使ってリグを提案するときに最初に送るリクエスト (送る前の確認に使う)。
fn rig_request(
    document: &Document,
    labels: &[LayerLabel],
    image: &[u8],
    ask: &RigAsk,
) -> Result<ProviderRequest, AiError> {
    if document.module().is_some() {
        let parts = module_mesh_parts(document, labels);
        module_rig_request_guided(document, &parts, image, &[], &ask.module()?)
    } else {
        let parts = mesh_parts(document.model(), labels);
        rig_request_guided(document.model(), &parts, image, &[], &ask.model()?)
    }
}

/// リグを提案する。`provider` があればテンプレートで決められない値を AI に決めさせ (注文と前の答えを添えて)、
/// なければ既定値。モジュールのプロジェクトならモジュール向けの提案 ([`module_rig`])。
pub(crate) fn rig(
    document: &Document,
    labels: &[LayerLabel],
    image: &[u8],
    provider: Option<&dyn Provider>,
    ask: &RigAsk,
) -> Result<Rigged, AiError> {
    if document.module().is_some() {
        return module_rig(document, labels, image, provider, ask);
    }
    let model = document.model();
    let guidance = ask.model()?;
    let guidance = &guidance;
    let parts = mesh_parts(model, labels);
    let options = RigOptions::default();
    let (mut proposal, previous, unapplied) = match provider {
        Some(p) => {
            let rig = propose_rig_with_ai_guided(
                p,
                document,
                &parts,
                image,
                &options,
                DEFAULT_MAX_ATTEMPTS,
                guidance,
            )?;
            let mut proposal = rig.proposal;
            if !rig.rejected.is_empty() {
                proposal.rationale.push_str(&format!(
                    "\nAsked the AI again {} time(s): its answer was out of range or did not pass the checks",
                    rig.rejected.len()
                ));
            }
            proposal.title.push_str(" — AI");
            let previous = PreviousRig {
                params: rig.params,
                reason: rig.reason,
            };
            (proposal, PreviousAnswer::Model(previous), rig.unapplied)
        }
        None => {
            let params = RigParams::default();
            let mut proposal = propose_rig(document, &parts, &params, &options)?;
            proposal.title.push_str(" — rules");
            let previous = PreviousRig {
                params,
                reason: TEMPLATE_DEFAULTS.into(),
            };
            (proposal, PreviousAnswer::Model(previous), Vec::new())
        }
    };
    proposal.rationale.push_str(&labels_note(labels));
    Ok(Rigged {
        proposal,
        previous,
        unapplied,
    })
}

/// AI を使わなかった提案の、前の答えの理由。
const TEMPLATE_DEFAULTS: &str = "The template defaults (no AI was used).";

/// モジュールのプロジェクトのリグ (`ugoe_ai::module_rig`): パラメータを足さず、衣装を本体の変形に追従させ、
/// 揺れるラベルの衣装に物理を付ける。
fn module_rig(
    document: &Document,
    labels: &[LayerLabel],
    image: &[u8],
    provider: Option<&dyn Provider>,
    ask: &RigAsk,
) -> Result<Rigged, AiError> {
    let parts = module_mesh_parts(document, labels);
    let (mut proposal, previous, unapplied) = match provider {
        Some(p) => {
            let rig = propose_module_rig_with_ai_guided(
                p,
                document,
                &parts,
                image,
                &ask.module.into(),
                DEFAULT_MAX_ATTEMPTS,
                &ask.module()?,
            )?;
            let mut proposal = rig.proposal;
            if !rig.rejected.is_empty() {
                proposal.rationale.push_str(&format!(
                    "\nAsked the AI again {} time(s): its answer was out of range or did not pass the checks",
                    rig.rejected.len()
                ));
            }
            proposal.title.push_str(" — AI");
            let previous = PreviousModuleRig {
                params: rig.params,
                reason: rig.reason,
            };
            (proposal, previous, rig.unapplied)
        }
        None => {
            let params = ModuleRigParams::default();
            let mut proposal = propose_module_rig(document, &parts, &params, &ask.module.into())?;
            proposal.title.push_str(" — rules");
            let previous = PreviousModuleRig {
                params,
                reason: TEMPLATE_DEFAULTS.into(),
            };
            (proposal, previous, Vec::new())
        }
    };
    proposal.rationale.push_str(&labels_note(labels));
    Ok(Rigged {
        proposal,
        previous: PreviousAnswer::Module(previous),
        unapplied,
    })
}

/// 提案の理由に添える、分類の要約 (確からしさが低いか、名前の規則と食い違うレイヤー)。
fn labels_note(labels: &[LayerLabel]) -> String {
    let options = ClassifyOptions::default();
    let review: Vec<String> = labels
        .iter()
        .filter(|l| l.needs_review(&options))
        .map(|l| {
            let by_name = l
                .name_disagreement
                .map(|d| {
                    let label = d.label.unwrap_or(l.label).as_str();
                    let side = d.side.unwrap_or(l.side).as_str();
                    format!("; the name says {label} {side}")
                })
                .unwrap_or_default();
            format!(
                "{} ({} {}{by_name})",
                l.layer_path.last().map_or("?", String::as_str),
                l.label.as_str(),
                l.side.as_str()
            )
        })
        .collect();
    if review.is_empty() {
        return String::new();
    }
    format!(
        "\nLayers classified with low confidence or against their names ({}; check them before \
         adopting): {}",
        review.len(),
        review.join(", ")
    )
}

/// 提案を作り始めたときの、開いているプロジェクトとドキュメントの状態。作る間 (AI の応答を待つ間など) に変わったら、
/// 結果を置かない。ドキュメントの状態の番号は開くたびに 0 から数え直すので、セッションの番号も比べる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Base {
    session: u64,
    state: u64,
}

impl Base {
    pub(crate) fn of(document: &Document, session: &ProjectSession) -> Self {
        Self {
            session: session.id(),
            state: document.state_id(),
        }
    }

    /// 今も同じプロジェクトの同じ状態か。
    fn check(self, document: &Document, session: &ProjectSession) -> IpcResult<()> {
        if Self::of(document, session) == self {
            Ok(())
        } else {
            Err(EditorError::ProposalModelChanged.into())
        }
    }

    /// ロックの外で作った編集を適用する前に、今も同じプロジェクトの同じ状態かを確かめる (提案ではない編集。
    /// `lighting::generate_texture_normal_map`・`psd::import_psd`)。変わっていれば [`EditorError::ModelChanged`]。
    pub(crate) fn check_edit(self, document: &Document, session: &ProjectSession) -> IpcResult<()> {
        if Self::of(document, session) == self {
            Ok(())
        } else {
            Err(EditorError::ModelChanged.into())
        }
    }
}

// ---- IPC ----

fn lock_document(state: &AppState) -> IpcResult<std::sync::MutexGuard<'_, Document>> {
    state
        .document
        .lock()
        .map_err(|_| EditorError::LockPoisoned { what: "document" }.into())
}

/// 提案の一覧・元にする PSD・分類の結果。
#[tauri::command]
pub fn get_proposals(state: State<'_, AppState>) -> IpcResult<ProposalState> {
    let document = lock_document(&state)?;
    Ok(lock_project(&state)?.proposals.state(&document))
}

/// PSD のレイヤーを分類し、結果を覚える (次の `propose_rig` の入力)。`psd` を省略すると、取り込んだ PSD。
/// `use_ai` なら画像を選んだ AI のプロバイダに送る (同意がなければ送らずに `needs_consent`)。偽なら名前の規則だけ。
/// `instructions` は AI に添える利用者の注文 (AI を使うときだけ送る)。`revise` なら、同じ PSD の今の分類を
/// 前の答えとして送り、注文が求めるところだけ直させる (分類の作り直し)。
#[tauri::command]
pub async fn classify_psd_layers(
    app: AppHandle,
    psd: Option<String>,
    use_ai: bool,
    instructions: Option<String>,
    revise: bool,
    consent: Option<ConsentAnswer>,
) -> IpcResult<AiReply<Classification>> {
    blocking(app, move |state| {
        // 選んだ PSD は、開くダイアログで選んだものか取り込んだもの (`crate::access`)
        if let Some(psd) = &psd {
            crate::access::readable_path(state, psd)?;
        }
        let ask = Ask {
            instructions: instructions.as_deref(),
            revise,
        };
        classify_guided_in(state, psd, use_ai, &ask, consent.as_ref())
    })
    .await
}

/// 分類の結果からリグ (顔の角度・目・口) を提案し、提案の一覧に置く (ドキュメントは変えない)。
/// `use_ai` なら原画を選んだ AI のプロバイダに送って値を決めさせる (答えが通らなければ問い直す)。偽ならテンプレートの既定値。
/// `instructions` は AI に添える利用者の注文 (AI を使うときだけ送る)。`module_options` はモジュールのプロジェクトの
/// リグの組み方 (省略すると既定。モデルのプロジェクトでは使わない)。
#[tauri::command]
pub async fn propose_rig_from_psd(
    app: AppHandle,
    use_ai: bool,
    instructions: Option<String>,
    consent: Option<ConsentAnswer>,
    module_options: Option<ModuleRigChoice>,
) -> IpcResult<AiReply<ProposalView>> {
    blocking(app, move |state| {
        propose_guided_in(
            state,
            use_ai,
            instructions.as_deref(),
            module_options.unwrap_or_default(),
            consent.as_ref(),
        )
    })
    .await
}

/// 提案を、前の答え (値と理由) と注文を AI に送って作り直す。新しい提案は前の提案を置き換えず、一覧の後ろに並べる
/// (前の提案は残る。見比べて選び、要らない方を却下する)。いつも AI を使う。分類は今の分類を使う (ラベルも直したいときは
/// 先に `classify_psd_layers` を `revise` で呼ぶ)。
#[tauri::command]
pub async fn regenerate_rig_proposal(
    app: AppHandle,
    id: u32,
    instructions: Option<String>,
    consent: Option<ConsentAnswer>,
    module_options: Option<ModuleRigChoice>,
) -> IpcResult<AiReply<ProposalView>> {
    blocking(app, move |state| {
        regenerate_in(
            state,
            id,
            instructions.as_deref(),
            module_options.unwrap_or_default(),
            consent.as_ref(),
        )
    })
    .await
}

/// 選んだまとまり (`choices`。まとまりと設定は `get_normal_map_groups` で得る) のテクスチャに、アルファの形から法線マップを
/// 作る提案を一覧に置く (ドキュメントは変えない)。AI は使わず、何も送らない。ラベルはメッシュの名前の規則に、分類が
/// あれば重ねる。`replace_existing` なら法線マップを持つテクスチャも作り直す。モデルのライティングが無効なら有効にする
/// コマンドも入る。コマンドはまとまりごとに区切る (`ProposalState::sections`)。
#[tauri::command]
pub async fn propose_normal_maps(
    app: AppHandle,
    choices: Vec<NormalMapGroupChoice>,
    replace_existing: bool,
) -> IpcResult<ProposalView> {
    blocking(app, move |state| {
        propose_normal_maps_in(state, &choices, replace_existing)
    })
    .await
}

/// [`propose_normal_maps`] の本体。
pub(crate) fn propose_normal_maps_in(
    state: &AppState,
    choices: &[NormalMapGroupChoice],
    replace_existing: bool,
) -> IpcResult<ProposalView> {
    // 作る間はロックを持たない (大きなテクスチャでは時間がかかる)。
    let (document, base, layers) = {
        let document = lock_document(state)?;
        let session = lock_project(state)?;
        let base = Base::of(&document, &session);
        (
            document.detached(),
            base,
            session.proposals.layer_labels(&document),
        )
    };
    let (proposal, sections) =
        normal_map_proposal_in(&document, layers.as_deref(), choices, replace_existing)?;
    let document = lock_document(state)?;
    let mut session = lock_project(state)?;
    base.check(&document, &session)?;
    let proposals = &mut session.proposals;
    let view = proposals.proposals.add(
        &document,
        proposal.title,
        proposal.rationale,
        proposal.commands,
    )?;
    proposals.sections.insert(view.id, sections);
    Ok(view)
}

/// 分類に添える注文と、分類の作り直しか。
#[derive(Debug, Default)]
pub(crate) struct Ask<'a> {
    pub instructions: Option<&'a str>,
    pub revise: bool,
}

/// [`classify_psd_layers`] の本体。
pub(crate) fn classify_guided_in(
    state: &AppState,
    psd: Option<String>,
    use_ai: bool,
    ask: &Ask<'_>,
    consent: Option<&ConsentAnswer>,
) -> IpcResult<AiReply<Classification>> {
    let (path, opened) = {
        let session = lock_project(state)?;
        let path = match psd {
            Some(p) => absolute_path(&p)?,
            None => session.proposals.psd.clone().ok_or(EditorError::NoPsd)?,
        };
        (path, session.id())
    };
    let (tree, identity) = read_psd(&path)?;
    let mut guidance = ClassifyGuidance::default();
    let labels = if use_ai {
        guidance.instructions = instructions(ask.instructions)?;
        if ask.revise {
            // 前の答えには利用者の直しを重ねたものを送る。
            let document = lock_document(state)?;
            let session = lock_project(state)?;
            let current = session
                .proposals
                .corrected(&document)
                .filter(|c| c.psd == path)
                .ok_or(EditorError::NotClassified)?;
            guidance.previous = Some(current.labels);
        }
        let requests =
            classification_requests_guided(&tree, &ClassifyOptions::default(), &guidance)
                .map_err(EditorError::Ai)?;
        let chars = instruction_chars(&guidance.instructions);
        match prepare_instructed(state, Purpose::Proposal, &requests, 0, chars, consent)? {
            Prepared::NeedsConsent(disclosure) => {
                return Ok(AiReply::NeedsConsent { disclosure });
            }
            Prepared::Ready(job) => {
                let result = classify(&tree, Some(job.provider()), &guidance);
                job.finish(result)
            }
        }
    } else {
        classify(&tree, None, &guidance)
    }
    .map_err(EditorError::Ai)?;
    let classified = Classified {
        psd: path.clone(),
        identity,
        labels,
        by_ai: use_ai,
        instructions: guidance.instructions,
    };
    let document = lock_document(state)?;
    let value = classified.view(document.layer_corrections());
    let mut session = lock_project(state)?;
    // 分類する間に別のプロジェクトに替わったら、そのプロジェクトには残さない
    if session.id() != opened {
        return Err(EditorError::ProposalModelChanged.into());
    }
    session.proposals.set_psd(path);
    session.proposals.classification = Some(classified);
    // 保存先があれば、開き直しても分類し直さずに済むようキャッシュに残す (ファイルはロックの外で書く)。
    let cache = session
        .path()
        .map(Path::to_path_buf)
        .zip(session.proposals.cache());
    drop(session);
    drop(document);
    if let Some((dir, cache)) = cache {
        write_cache(&dir, &cache);
    }
    Ok(AiReply::Done { value })
}

/// [`propose_rig_from_psd`] の本体。
pub(crate) fn propose_guided_in(
    state: &AppState,
    use_ai: bool,
    instructions_text: Option<&str>,
    module: ModuleRigChoice,
    consent: Option<&ConsentAnswer>,
) -> IpcResult<AiReply<ProposalView>> {
    let ask = RigAsk {
        instructions: if use_ai {
            instructions(instructions_text)?
        } else {
            None
        },
        module,
        ..RigAsk::default()
    };
    make_rig(state, use_ai, ask, None, consent)
}

/// [`regenerate_rig_proposal`] の本体。
pub(crate) fn regenerate_in(
    state: &AppState,
    id: u32,
    instructions_text: Option<&str>,
    module: ModuleRigChoice,
    consent: Option<&ConsentAnswer>,
) -> IpcResult<AiReply<ProposalView>> {
    let previous = lock_project(state)?
        .proposals
        .rigs
        .get(&id)
        .cloned()
        .ok_or(EditorError::NotRegenerable(id))?;
    let ask = RigAsk {
        instructions: instructions(instructions_text)?,
        previous: Some(previous),
        module,
        ..RigAsk::default()
    };
    make_rig(state, true, ask, Some(id), consent)
}

/// 今の分類からリグを提案して一覧に置く。`revises` は作り直した元の提案。
fn make_rig(
    state: &AppState,
    use_ai: bool,
    mut guidance: RigAsk,
    revises: Option<u32>,
    consent: Option<&ConsentAnswer>,
) -> IpcResult<AiReply<ProposalView>> {
    // 提案を作る間はロックを持たない (AI の応答を待つ)。複製は履歴を持たず、モジュールの情報は持つ。
    let (document, base, classified) = {
        let document = lock_document(state)?;
        let session = lock_project(state)?;
        // 利用者の直しを重ねた分類から作る。
        let classified = session
            .proposals
            .corrected(&document)
            .ok_or(EditorError::NotClassified)?;
        (
            document.detached(),
            Base::of(&document, &session),
            classified,
        )
    };
    // 同じ注文で AI が分類したなら、ラベルに関わる注文は分類で反映した。
    guidance.labels_follow_instructions = classified.by_ai
        && guidance.instructions.is_some()
        && classified.instructions == guidance.instructions;
    let rigged = if use_ai {
        let tree = read_tree(&classified.psd)?;
        let image = overview(&tree).map_err(EditorError::Ai)?;
        let request = rig_request(&document, &classified.labels, &image, &guidance)
            .map_err(EditorError::Ai)?;
        let retries = DEFAULT_MAX_ATTEMPTS - 1;
        let chars = instruction_chars(&guidance.instructions);
        match prepare_instructed(
            state,
            Purpose::Proposal,
            &[request],
            retries,
            chars,
            consent,
        )? {
            Prepared::NeedsConsent(disclosure) => {
                return Ok(AiReply::NeedsConsent { disclosure });
            }
            Prepared::Ready(job) => {
                let result = rig(
                    &document,
                    &classified.labels,
                    &image,
                    Some(job.provider()),
                    &guidance,
                );
                job.finish(result)
            }
        }
    } else {
        rig(&document, &classified.labels, &[], None, &guidance)
    }
    .map_err(EditorError::Ai)?;
    let origin = ProposalOrigin {
        revises,
        instructions: guidance.instructions.map(|i| i.as_str().to_owned()),
        unapplied: rigged.unapplied.clone(),
    };
    let view = place(state, base, rigged, origin)?;
    Ok(AiReply::Done { value: view })
}

/// 作ったリグの提案を一覧に置き、作り直しに使う値と理由を覚える。提案を作る間にプロジェクトかドキュメントが変わっていたら
/// 置かない。
pub(crate) fn place(
    state: &AppState,
    base: Base,
    rigged: Rigged,
    origin: ProposalOrigin,
) -> IpcResult<ProposalView> {
    let document = lock_document(state)?;
    let mut session = lock_project(state)?;
    base.check(&document, &session)?;
    let proposals = &mut session.proposals;
    let writes = matches!(rigged.previous, PreviousAnswer::Module(_))
        .then(|| mesh_writes(&rigged.proposal.commands));
    let view = proposals.proposals.add(
        &document,
        rigged.proposal.title,
        rigged.proposal.rationale,
        rigged.proposal.commands,
    )?;
    proposals.proposals.set_origin(view.id, origin);
    if let Some(writes) = writes {
        proposals.writes.insert(view.id, writes);
    }
    proposals.rigs.insert(view.id, rigged.previous);
    let view = proposals
        .proposals
        .views(&document)
        .into_iter()
        .find(|v| v.id == view.id)
        .ok_or(EditorError::ProposalDisappeared)?;
    Ok(view)
}

/// `excluded` の番号のコマンドを除いた提案を、ドキュメントの複製に適用した姿を返す (ドキュメントは変えない)。
/// 適用・検証に失敗したらエラー (一部だけの採用で、使う要素を作るコマンドを外したときなど)。
#[tauri::command]
pub async fn preview_proposal(
    app: AppHandle,
    id: u32,
    excluded: Vec<usize>,
) -> IpcResult<ProposalPreview> {
    blocking(app, move |state| {
        let document = lock_document(state)?;
        let mut session = lock_project(state)?;
        session.proposals.preview = None;
        let (preview, report) = session
            .proposals
            .proposals
            .preview(&document, id, &excluded)?;
        let model_json = ugoe_format::model_to_json(preview.model())?;
        session.proposals.preview = Some(preview);
        Ok(ProposalPreview { model_json, report })
    })
    .await
}

/// 最後にプレビューした姿の `.ugoe` バンドル (テクスチャ付き)。生のバイト列で送る。
#[tauri::command]
pub async fn get_proposal_preview_bundle(app: AppHandle) -> IpcResult<tauri::ipc::Response> {
    blocking(app, |state| {
        // ロックの中では写しを取るだけにし、バンドル (PNG を含む) はロックの外で作る。
        let project = {
            let session = lock_project(state)?;
            let preview = session
                .proposals
                .preview
                .as_ref()
                .ok_or(EditorError::NoPreview)?;
            session.to_project(preview)
        };
        let bytes = ugoe_editor_core::export_bundle(&project)?;
        Ok(tauri::ipc::Response::new(bytes))
    })
    .await
}

/// プレビューの複製を捨てる (プレビューをやめたとき)。
#[tauri::command]
pub fn clear_proposal_preview(state: State<'_, AppState>) -> IpcResult<()> {
    lock_project(&state)?.proposals.preview = None;
    Ok(())
}

/// 提案を採用する: `excluded` の番号のコマンドを除いて `Batch` として適用し (1 回の Undo で戻る)、一覧から隠す
/// (Undo で提案を作った状態に戻れば再び出る)。
#[tauri::command]
pub fn adopt_proposal(
    state: State<'_, AppState>,
    id: u32,
    excluded: Vec<usize>,
) -> IpcResult<ApplyReport> {
    adopt_in(&state, id, &excluded)
}

/// [`adopt_proposal`] の本体。モジュールのリグの提案なら、キーフォームを書いたメッシュの形を覚える。
pub(crate) fn adopt_in(state: &AppState, id: u32, excluded: &[usize]) -> IpcResult<ApplyReport> {
    let mut document = lock_document(state)?;
    let mut session = lock_project(state)?;
    let report = session
        .proposals
        .proposals
        .adopt(&mut document, id, excluded)?;
    session.proposals.preview = None;
    session.proposals.remember_rigged(&document, id, excluded);
    let rigged = session.proposals.rigged_value();
    session.set_editor_value(RIGGED_MESHES_KEY, rigged);
    Ok(report)
}

/// 本体の変形を写した後に形を編集した衣装のメッシュの知らせを消す (今の形を覚え直す。提案し直さずに済ませるとき)。
/// 覚え直した形はプロジェクトに残す (保存するまでファイルには書かない)。
#[tauri::command]
pub fn dismiss_stale_rigs(state: State<'_, AppState>) -> IpcResult<ProposalState> {
    dismiss_stale_rigs_in(&state)
}

/// [`dismiss_stale_rigs`] の本体。
pub(crate) fn dismiss_stale_rigs_in(state: &AppState) -> IpcResult<ProposalState> {
    let document = lock_document(state)?;
    let mut session = lock_project(state)?;
    session.proposals.forget_stale(&document);
    let rigged = session.proposals.rigged_value();
    session.set_editor_value(RIGGED_MESHES_KEY, rigged);
    Ok(session.proposals.state(&document))
}

/// 提案を捨てる。
#[tauri::command]
pub fn reject_proposal(state: State<'_, AppState>, id: u32) -> IpcResult<ProposalState> {
    reject_in(&state, id)
}

/// [`reject_proposal`] の本体。作り直しに使う値と理由も捨てる。
pub(crate) fn reject_in(state: &AppState, id: u32) -> IpcResult<ProposalState> {
    let document = lock_document(state)?;
    let mut session = lock_project(state)?;
    session.proposals.proposals.reject(id)?;
    session.proposals.rigs.remove(&id);
    session.proposals.sections.remove(&id);
    session.proposals.writes.remove(&id);
    session.proposals.preview = None;
    Ok(session.proposals.state(&document))
}

#[cfg(test)]
#[path = "proposals_tests.rs"]
mod tests;
