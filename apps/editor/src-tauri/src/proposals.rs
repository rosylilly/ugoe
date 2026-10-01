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
//!   ([`ProposalState::stale_rigs`]。覚えるのはこのセッションの間だけ)。

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use ts_rs::TS;
use ugoe_ai::classify::{
    ClassifyGuidance, ClassifyOptions, LayerLabel, classification_requests_guided,
    classify_by_name, classify_layers_guided, overview_png,
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
use ugoe_editor_core::{
    ApplyReport, Command, Document, KeyformTarget, ProposalOrigin, ProposalView, Proposals,
};
use ugoe_model::{DeformerId, Mesh, MeshId};
use ugoe_psd::LayerTree;

use crate::AppState;
use crate::ai::{AiReply, ConsentAnswer, Prepared, blocking, lock_project, prepare_instructed};
use crate::ai_provider::Purpose;
use crate::lighting::{CommandSection, NormalMapGroupChoice, normal_map_proposal_in};
use crate::project::absolute_path;

/// 提案の元にする PSD がないときの断りの文言。
const NO_PSD: &str = "no PSD to classify: choose the PSD this model was imported from";

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
    /// 採用したモジュールのリグの提案がキーフォームを書いた衣装のメッシュと、採用したときの形。
    rigged: BTreeMap<MeshId, MeshShape>,
}

/// 本体の変形を写したメッシュの形 (これが変わったら、写した変形は合わない)。
#[derive(Debug, Clone, PartialEq)]
struct MeshShape {
    parent: Option<DeformerId>,
    uvs: Vec<[u32; 2]>,
    indices: Vec<u16>,
}

impl MeshShape {
    fn of(mesh: &Mesh) -> Self {
        Self {
            parent: mesh.parent.clone(),
            uvs: mesh
                .uvs
                .iter()
                .map(|u| [u.x.to_bits(), u.y.to_bits()])
                .collect(),
            indices: mesh.indices.clone(),
        }
    }
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

    /// 最後に分類した PSD のレイヤーのラベル (法線マップのまとまりに使う。`crate::lighting`)。
    pub(crate) fn layer_labels(&self) -> Option<Vec<LayerLabel>> {
        self.classification.as_ref().map(|c| c.labels.clone())
    }

    /// 提案の置き場 (エディタにつないだエージェントの提案もここに置く。`crate::mcp`)。
    pub(crate) fn proposals_mut(&mut self) -> &mut Proposals {
        &mut self.proposals
    }

    /// 採用したモジュールのリグの提案がキーフォームを書いた衣装のメッシュのうち、その後に形を編集したもの。
    /// 今キーフォームの軸を持たないもの (採用を Undo したもの) と、消えたものは除く。
    fn stale_rigs(&self, document: &Document) -> Vec<StaleRig> {
        let model = document.model();
        self.rigged
            .iter()
            .filter_map(|(id, shape)| {
                let mesh = model.meshes.iter().find(|m| &m.id == id)?;
                let stale = !mesh.keyforms.bindings.is_empty() && MeshShape::of(mesh) != *shape;
                stale.then(|| StaleRig {
                    mesh: id.to_string(),
                    name: mesh.name.clone(),
                })
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
                self.rigged.insert(mesh.clone(), MeshShape::of(found));
            }
        }
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
            classification: self.classification.as_ref().map(Classified::view),
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
    /// 確からしさが低く、人間が確かめたほうがよい。
    pub needs_review: bool,
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
}

impl Classified {
    fn view(&self) -> Classification {
        let options = ClassifyOptions::default();
        Classification {
            psd: self.psd.display().to_string(),
            by_ai: self.by_ai,
            instructions: self.instructions.as_ref().map(|i| i.as_str().to_owned()),
            layers: self
                .labels
                .iter()
                .map(|l| LayerLabelView {
                    path: l.layer_path.clone(),
                    label: l.label.as_str().to_owned(),
                    side: l.side.as_str().to_owned(),
                    confidence: l.confidence,
                    needs_review: l.needs_review(&options),
                })
                .collect(),
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

fn read_tree(path: &Path) -> Result<LayerTree, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ugoe_psd::import(&bytes).map_err(|e| format!("{}: {e}", path.display()))
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
fn instructions(text: Option<&str>) -> Result<Option<Instructions>, String> {
    Instructions::new(text.unwrap_or_default()).map_err(|e| e.to_string())
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
                model,
                &parts,
                image,
                &options,
                DEFAULT_MAX_ATTEMPTS,
                guidance,
            )?;
            let mut proposal = rig.proposal;
            if !rig.rejected.is_empty() {
                proposal.rationale.push_str(&format!(
                    "\nAI の答えのうち {} 回分は範囲の外か検証を通らなかったので問い直した",
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
            let mut proposal = propose_rig(model, &parts, &params, &options)?;
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
                    "\nAI の答えのうち {} 回分は範囲の外か検証を通らなかったので問い直した",
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

/// 提案の理由に添える、分類の要約 (確からしさが低いレイヤー)。
fn labels_note(labels: &[LayerLabel]) -> String {
    let options = ClassifyOptions::default();
    let review: Vec<String> = labels
        .iter()
        .filter(|l| l.needs_review(&options))
        .map(|l| {
            format!(
                "{} ({})",
                l.layer_path.last().map_or("?", String::as_str),
                l.label.as_str()
            )
        })
        .collect();
    if review.is_empty() {
        return String::new();
    }
    format!(
        "\n分類の確からしさが低いレイヤー ({} 枚。確かめてから採用する): {}",
        review.len(),
        review.join(", ")
    )
}

// ---- IPC ----

fn lock_document(state: &AppState) -> Result<std::sync::MutexGuard<'_, Document>, String> {
    state
        .document
        .lock()
        .map_err(|_| "document lock poisoned".to_string())
}

/// 提案の一覧・元にする PSD・分類の結果。
#[tauri::command]
pub fn get_proposals(state: State<'_, AppState>) -> Result<ProposalState, String> {
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
) -> Result<AiReply<Classification>, String> {
    blocking(app, move |state| {
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
) -> Result<AiReply<ProposalView>, String> {
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
) -> Result<AiReply<ProposalView>, String> {
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
) -> Result<ProposalView, String> {
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
) -> Result<ProposalView, String> {
    let layers = lock_project(state)?.proposals.layer_labels();
    // 作る間はロックを持たない (大きなテクスチャでは時間がかかる)。
    let (document, base) = {
        let document = lock_document(state)?;
        (document.detached(), document.state_id())
    };
    let (proposal, sections) =
        normal_map_proposal_in(&document, layers.as_deref(), choices, replace_existing)?;
    let document = lock_document(state)?;
    if document.state_id() != base {
        return Err("the model changed while the proposal was being made; propose again".into());
    }
    let mut session = lock_project(state)?;
    let proposals = &mut session.proposals;
    let view = proposals
        .proposals
        .add(
            &document,
            proposal.title,
            proposal.rationale,
            proposal.commands,
        )
        .map_err(|e| e.to_string())?;
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
) -> Result<AiReply<Classification>, String> {
    let path = match psd {
        Some(p) => absolute_path(&p)?,
        None => lock_project(state)?.proposals.psd.clone().ok_or(NO_PSD)?,
    };
    let tree = read_tree(&path)?;
    let mut guidance = ClassifyGuidance::default();
    let labels = if use_ai {
        guidance.instructions = instructions(ask.instructions)?;
        if ask.revise {
            let session = lock_project(state)?;
            let current = session
                .proposals
                .classification
                .as_ref()
                .filter(|c| c.psd == path)
                .ok_or("classify the PSD layers first")?;
            guidance.previous = Some(current.labels.clone());
        }
        let requests =
            classification_requests_guided(&tree, &ClassifyOptions::default(), &guidance)
                .map_err(|e| e.to_string())?;
        let chars = instruction_chars(&guidance.instructions);
        match prepare_instructed(state, Purpose::Proposal, &requests, 0, chars, consent)? {
            Prepared::NeedsConsent(disclosure) => {
                return Ok(AiReply::NeedsConsent { disclosure });
            }
            Prepared::Ready(provider) => classify(&tree, Some(provider.as_ref()), &guidance),
        }
    } else {
        classify(&tree, None, &guidance)
    }
    .map_err(|e| e.to_string())?;
    let classified = Classified {
        psd: path.clone(),
        labels,
        by_ai: use_ai,
        instructions: guidance.instructions,
    };
    let value = classified.view();
    let mut session = lock_project(state)?;
    session.proposals.set_psd(path);
    session.proposals.classification = Some(classified);
    Ok(AiReply::Done { value })
}

/// [`propose_rig_from_psd`] の本体。
pub(crate) fn propose_guided_in(
    state: &AppState,
    use_ai: bool,
    instructions_text: Option<&str>,
    module: ModuleRigChoice,
    consent: Option<&ConsentAnswer>,
) -> Result<AiReply<ProposalView>, String> {
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
) -> Result<AiReply<ProposalView>, String> {
    let previous = lock_project(state)?
        .proposals
        .rigs
        .get(&id)
        .cloned()
        .ok_or_else(|| {
            format!("proposal {id} was not made in this panel, so it cannot be regenerated")
        })?;
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
) -> Result<AiReply<ProposalView>, String> {
    // 提案を作る間はロックを持たない (AI の応答を待つ)。複製は履歴を持たず、モジュールの情報は持つ。
    let (document, base) = {
        let document = lock_document(state)?;
        (document.detached(), document.state_id())
    };
    let classified = lock_project(state)?
        .proposals
        .classification
        .clone()
        .ok_or("classify the PSD layers first")?;
    // 同じ注文で AI が分類したなら、ラベルに関わる注文は分類で反映した。
    guidance.labels_follow_instructions = classified.by_ai
        && guidance.instructions.is_some()
        && classified.instructions == guidance.instructions;
    let rigged = if use_ai {
        let tree = read_tree(&classified.psd)?;
        let image = overview(&tree).map_err(|e| e.to_string())?;
        let request = rig_request(&document, &classified.labels, &image, &guidance)
            .map_err(|e| e.to_string())?;
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
            Prepared::Ready(provider) => rig(
                &document,
                &classified.labels,
                &image,
                Some(provider.as_ref()),
                &guidance,
            ),
        }
    } else {
        rig(&document, &classified.labels, &[], None, &guidance)
    }
    .map_err(|e| e.to_string())?;
    let origin = ProposalOrigin {
        revises,
        instructions: guidance.instructions.map(|i| i.as_str().to_owned()),
        unapplied: rigged.unapplied.clone(),
    };
    let view = place(state, base, rigged, origin)?;
    Ok(AiReply::Done { value: view })
}

/// 作ったリグの提案を一覧に置き、作り直しに使う値と理由を覚える。提案を作る間にドキュメントが変わっていたら置かない。
pub(crate) fn place(
    state: &AppState,
    base: u64,
    rigged: Rigged,
    origin: ProposalOrigin,
) -> Result<ProposalView, String> {
    let document = lock_document(state)?;
    if document.state_id() != base {
        return Err("the model changed while the proposal was being made; propose again".into());
    }
    let mut session = lock_project(state)?;
    let proposals = &mut session.proposals;
    let writes = matches!(rigged.previous, PreviousAnswer::Module(_))
        .then(|| mesh_writes(&rigged.proposal.commands));
    let view = proposals
        .proposals
        .add(
            &document,
            rigged.proposal.title,
            rigged.proposal.rationale,
            rigged.proposal.commands,
        )
        .map_err(|e| e.to_string())?;
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
        .ok_or("the proposal disappeared")?;
    Ok(view)
}

/// `excluded` の番号のコマンドを除いた提案を、ドキュメントの複製に適用した姿を返す (ドキュメントは変えない)。
/// 適用・検証に失敗したらエラー (一部だけの採用で、使う要素を作るコマンドを外したときなど)。
#[tauri::command]
pub fn preview_proposal(
    state: State<'_, AppState>,
    id: u32,
    excluded: Vec<usize>,
) -> Result<ProposalPreview, String> {
    let document = lock_document(&state)?;
    let mut session = lock_project(&state)?;
    session.proposals.preview = None;
    let (preview, report) = session
        .proposals
        .proposals
        .preview(&document, id, &excluded)
        .map_err(|e| e.to_string())?;
    let model_json = ugoe_format::model_to_json(preview.model()).map_err(|e| e.to_string())?;
    session.proposals.preview = Some(preview);
    Ok(ProposalPreview { model_json, report })
}

/// 最後にプレビューした姿の `.ugoe` バンドル (テクスチャ付き)。生のバイト列で送る。
#[tauri::command]
pub fn get_proposal_preview_bundle(
    state: State<'_, AppState>,
) -> Result<tauri::ipc::Response, String> {
    let session = lock_project(&state)?;
    let preview = session
        .proposals
        .preview
        .as_ref()
        .ok_or("no proposal is being previewed")?;
    let bytes =
        ugoe_editor_core::export_bundle(&session.to_project(preview)).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// プレビューの複製を捨てる (プレビューをやめたとき)。
#[tauri::command]
pub fn clear_proposal_preview(state: State<'_, AppState>) -> Result<(), String> {
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
) -> Result<ApplyReport, String> {
    adopt_in(&state, id, &excluded)
}

/// [`adopt_proposal`] の本体。モジュールのリグの提案なら、キーフォームを書いたメッシュの形を覚える。
pub(crate) fn adopt_in(
    state: &AppState,
    id: u32,
    excluded: &[usize],
) -> Result<ApplyReport, String> {
    let mut document = lock_document(state)?;
    let mut session = lock_project(state)?;
    let report = session
        .proposals
        .proposals
        .adopt(&mut document, id, excluded)
        .map_err(|e| e.to_string())?;
    session.proposals.preview = None;
    session.proposals.remember_rigged(&document, id, excluded);
    Ok(report)
}

/// 本体の変形を写した後に形を編集した衣装のメッシュの知らせを消す (今の形を覚え直す。提案し直さずに済ませるとき)。
#[tauri::command]
pub fn dismiss_stale_rigs(state: State<'_, AppState>) -> Result<ProposalState, String> {
    dismiss_stale_rigs_in(&state)
}

/// [`dismiss_stale_rigs`] の本体。
pub(crate) fn dismiss_stale_rigs_in(state: &AppState) -> Result<ProposalState, String> {
    let document = lock_document(state)?;
    let mut session = lock_project(state)?;
    let proposals = &mut session.proposals;
    for (id, shape) in proposals.rigged.iter_mut() {
        if let Some(mesh) = document.model().meshes.iter().find(|m| &m.id == id) {
            *shape = MeshShape::of(mesh);
        }
    }
    Ok(proposals.state(&document))
}

/// 提案を捨てる。
#[tauri::command]
pub fn reject_proposal(state: State<'_, AppState>, id: u32) -> Result<ProposalState, String> {
    reject_in(&state, id)
}

/// [`reject_proposal`] の本体。作り直しに使う値と理由も捨てる。
pub(crate) fn reject_in(state: &AppState, id: u32) -> Result<ProposalState, String> {
    let document = lock_document(state)?;
    let mut session = lock_project(state)?;
    session
        .proposals
        .proposals
        .reject(id)
        .map_err(|e| e.to_string())?;
    session.proposals.rigs.remove(&id);
    session.proposals.sections.remove(&id);
    session.proposals.writes.remove(&id);
    session.proposals.preview = None;
    Ok(session.proposals.state(&document))
}

#[cfg(test)]
#[path = "proposals_tests.rs"]
mod tests;
