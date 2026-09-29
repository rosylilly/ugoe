//! AI の提案 (PSD のレイヤーの分類 → 自動リギング) と、提案のプレビュー・採用・却下の IPC コマンド。
//! 方針: ADR 0006・`docs/editor.md#ai-の提案`。提案の置き場・プレビュー・採用の規則は
//! `ugoe_editor_core::proposal` にあり、ここは入力 (PSD・分類) を集めて ugoe-ai を呼び、結果を渡すだけ。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。
//!
//! - 外部に送るもの (`use_ai`) は必ず [`crate::ai::prepare_claude_requests`] を通し、同意がなければ送らずに
//!   `AiReply::NeedsConsent` を返す。AI を使わない経路 (名前の規則とテンプレートの既定値) は何も送らない。
//! - 提案は採用するまでドキュメントを変えない。プレビューは複製に適用したもの ([`ProposalSession`] が持つ)。

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, State};
use ts_rs::TS;
use ugoe_ai::classify::{
    ClassifyOptions, LayerLabel, classification_requests, classify_by_name, classify_layers,
    overview_png,
};
use ugoe_ai::claude::ClaudeConfig;
use ugoe_ai::rig::{
    DEFAULT_MAX_ATTEMPTS, RigOptions, RigParams, mesh_parts, propose_rig, propose_rig_with_ai,
    rig_request,
};
use ugoe_ai::{AiError, Proposal, Provider};
use ugoe_editor_core::{ApplyReport, Document, ProposalView, Proposals};
use ugoe_model::Model;
use ugoe_psd::LayerTree;

use crate::AppState;
use crate::ai::{
    AiReply, ConsentAnswer, Prepared, blocking, lock_project, prepare_claude_requests,
};
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
}

#[derive(Debug, Clone)]
struct Classified {
    psd: PathBuf,
    labels: Vec<LayerLabel>,
    by_ai: bool,
}

impl ProposalSession {
    /// 提案の元にする PSD を覚える。前と違う PSD なら、前の分類を捨てる。
    pub(crate) fn set_psd(&mut self, path: PathBuf) {
        if self.classification.as_ref().is_some_and(|c| c.psd != path) {
            self.classification = None;
        }
        self.psd = Some(path);
    }

    fn state(&self, document: &Document) -> ProposalState {
        ProposalState {
            proposals: self.proposals.views(document),
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
    pub layers: Vec<LayerLabelView>,
}

impl Classified {
    fn view(&self) -> Classification {
        let options = ClassifyOptions::default();
        Classification {
            psd: self.psd.display().to_string(),
            by_ai: self.by_ai,
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

/// レイヤーを分類する。`provider` があれば AI で、なければ名前の規則で。
pub(crate) fn classify(
    tree: &LayerTree,
    provider: Option<&dyn Provider>,
) -> Result<Vec<LayerLabel>, AiError> {
    match provider {
        Some(p) => classify_layers(p, tree, &ClassifyOptions::default()),
        None => Ok(classify_by_name(tree)),
    }
}

/// 原画の PNG (リギングの値を決めるために AI に送る)。
fn overview(tree: &LayerTree) -> Result<Vec<u8>, AiError> {
    overview_png(tree, ClassifyOptions::default().overview_max_side)
}

/// リグを提案する。`provider` があればテンプレートで決められない値を AI に決めさせ、なければ既定値。
pub(crate) fn rig(
    model: &Model,
    labels: &[LayerLabel],
    image: &[u8],
    provider: Option<&dyn Provider>,
) -> Result<Proposal, AiError> {
    let parts = mesh_parts(model, labels);
    let options = RigOptions::default();
    let mut proposal = match provider {
        Some(p) => {
            let rig = propose_rig_with_ai(p, model, &parts, image, &options, DEFAULT_MAX_ATTEMPTS)?;
            let mut proposal = rig.proposal;
            if !rig.rejected.is_empty() {
                proposal.rationale.push_str(&format!(
                    "\nAI の答えのうち {} 回分は範囲の外か検証を通らなかったので問い直した",
                    rig.rejected.len()
                ));
            }
            proposal.title.push_str(" — AI");
            proposal
        }
        None => {
            let mut proposal = propose_rig(model, &parts, &RigParams::default(), &options)?;
            proposal.title.push_str(" — rules");
            proposal
        }
    };
    proposal.rationale.push_str(&labels_note(labels));
    Ok(proposal)
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
/// `use_ai` なら画像を Claude API に送る (同意がなければ送らずに `needs_consent`)。偽なら名前の規則だけ。
#[tauri::command]
pub async fn classify_psd_layers(
    app: AppHandle,
    psd: Option<String>,
    use_ai: bool,
    consent: Option<ConsentAnswer>,
) -> Result<AiReply<Classification>, String> {
    blocking(app, move |state| {
        classify_in(state, psd, use_ai, consent.as_ref())
    })
    .await
}

/// 分類の結果からリグ (顔の角度・目・口) を提案し、提案の一覧に置く (ドキュメントは変えない)。
/// `use_ai` なら原画を Claude API に送って値を決めさせる (答えが通らなければ問い直す)。偽ならテンプレートの既定値。
#[tauri::command]
pub async fn propose_rig_from_psd(
    app: AppHandle,
    use_ai: bool,
    consent: Option<ConsentAnswer>,
) -> Result<AiReply<ProposalView>, String> {
    blocking(app, move |state| {
        propose_in(state, use_ai, consent.as_ref())
    })
    .await
}

/// [`classify_psd_layers`] の本体。
pub(crate) fn classify_in(
    state: &AppState,
    psd: Option<String>,
    use_ai: bool,
    consent: Option<&ConsentAnswer>,
) -> Result<AiReply<Classification>, String> {
    let path = match psd {
        Some(p) => absolute_path(&p)?,
        None => lock_project(state)?.proposals.psd.clone().ok_or(NO_PSD)?,
    };
    let tree = read_tree(&path)?;
    let labels = if use_ai {
        let requests = classification_requests(&tree, &ClassifyOptions::default())
            .map_err(|e| e.to_string())?;
        let config = ClaudeConfig::default();
        match prepare_claude_requests(state, config, &requests, 0, consent)? {
            Prepared::NeedsConsent(disclosure) => {
                return Ok(AiReply::NeedsConsent { disclosure });
            }
            Prepared::Ready(provider) => classify(&tree, Some(&provider)),
        }
    } else {
        classify(&tree, None)
    }
    .map_err(|e| e.to_string())?;
    let classified = Classified {
        psd: path.clone(),
        labels,
        by_ai: use_ai,
    };
    let value = classified.view();
    let mut session = lock_project(state)?;
    session.proposals.set_psd(path);
    session.proposals.classification = Some(classified);
    Ok(AiReply::Done { value })
}

/// [`propose_rig_from_psd`] の本体。
pub(crate) fn propose_in(
    state: &AppState,
    use_ai: bool,
    consent: Option<&ConsentAnswer>,
) -> Result<AiReply<ProposalView>, String> {
    let (model, base) = {
        let document = lock_document(state)?;
        (document.model().clone(), document.state_id())
    };
    let classified = lock_project(state)?
        .proposals
        .classification
        .clone()
        .ok_or("classify the PSD layers first")?;
    let proposal = if use_ai {
        let tree = read_tree(&classified.psd)?;
        let image = overview(&tree).map_err(|e| e.to_string())?;
        let parts = mesh_parts(&model, &classified.labels);
        let request = rig_request(&model, &parts, &image, &[]).map_err(|e| e.to_string())?;
        let config = ClaudeConfig::default();
        let retries = DEFAULT_MAX_ATTEMPTS - 1;
        match prepare_claude_requests(state, config, &[request], retries, consent)? {
            Prepared::NeedsConsent(disclosure) => {
                return Ok(AiReply::NeedsConsent { disclosure });
            }
            Prepared::Ready(provider) => rig(&model, &classified.labels, &image, Some(&provider)),
        }
    } else {
        rig(&model, &classified.labels, &[], None)
    }
    .map_err(|e| e.to_string())?;
    let document = lock_document(state)?;
    if document.state_id() != base {
        return Err("the model changed while the proposal was being made; propose again".into());
    }
    let view = lock_project(state)?
        .proposals
        .proposals
        .add(
            &document,
            proposal.title,
            proposal.rationale,
            proposal.commands,
        )
        .map_err(|e| e.to_string())?;
    Ok(AiReply::Done { value: view })
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
    let mut document = lock_document(&state)?;
    let mut session = lock_project(&state)?;
    let report = session
        .proposals
        .proposals
        .adopt(&mut document, id, &excluded)
        .map_err(|e| e.to_string())?;
    session.proposals.preview = None;
    Ok(report)
}

/// 提案を捨てる。
#[tauri::command]
pub fn reject_proposal(state: State<'_, AppState>, id: u32) -> Result<ProposalState, String> {
    let document = lock_document(&state)?;
    let mut session = lock_project(&state)?;
    session
        .proposals
        .proposals
        .reject(id)
        .map_err(|e| e.to_string())?;
    session.proposals.preview = None;
    Ok(session.proposals.state(&document))
}

#[cfg(test)]
#[path = "proposals_tests.rs"]
mod tests;
