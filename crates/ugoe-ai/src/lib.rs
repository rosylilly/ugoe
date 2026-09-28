//! AI 連携。設計: `docs/ai-integration.md`。
//!
//! 原則:
//! - AI は **[`ugoe_editor_core::Command`] の列を提案するだけ** で、モデルを直接書き換えない。
//!   提案はエディタで差分プレビューされ、ユーザーが採用して初めて適用される。
//! - LLM / 画像モデルの呼び出しは [`Provider`] トレイトの裏に隠し、ネットワーク無しで
//!   テストできるようにする (テストでは録画済み応答の FakeProvider を使う)。

use serde::{Deserialize, Serialize};
use ugoe_editor_core::Command;

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("model returned an invalid proposal: {0}")]
    InvalidProposal(String),
}

/// 画像とテキストを入力に構造化 JSON を返すモデルの抽象。
pub trait Provider {
    /// `schema` (JSON Schema) に従う JSON を返させる。
    fn complete_json(
        &self,
        request: &ProviderRequest,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, AiError>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub system: String,
    pub prompt: String,
    /// PNG バイト列。
    #[serde(skip)]
    pub images: Vec<Vec<u8>>,
}

/// AI からの提案。ユーザー承認前はモデルに適用されない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub title: String,
    pub rationale: String,
    pub commands: Vec<Command>,
}

// TODO(T0503): レイヤー意味分類 (`classify_layers`)。
// TODO(T0504): 自動パラメータ・キーフォーム設定 (`propose_rig`)。
