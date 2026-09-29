//! AI 連携。設計: `docs/ai-integration.md`。
//!
//! 原則:
//! - AI は **[`ugoe_editor_core::Command`] の列を提案するだけ** で、モデルを直接書き換えない。
//!   提案はエディタで差分プレビューされ、ユーザーが採用して初めて適用される。
//! - LLM / 画像モデルの呼び出しは [`Provider`] トレイトの裏に隠し、ネットワーク無しで
//!   テストできるようにする (テストでは [`FakeProvider`] か、録画した応答を再生する
//!   [`transport::ReplayTransport`] を使う)。
//! - 既定のプロバイダは Anthropic の Claude API ([`claude::ClaudeProvider`])。

pub mod classify;
pub mod claude;
pub mod depth;
mod fake;
pub mod transport;
pub mod turn;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use ugoe_editor_core::Command;

pub use fake::FakeProvider;

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("model returned an invalid proposal: {0}")]
    InvalidProposal(String),
    /// API キーが見つからない・空である。
    #[error("API key is not configured: {0}")]
    MissingApiKey(String),
    /// 送る前に分かる入力の誤り (画像が PNG でない、など)。
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// API がエラーを返した (再試行しても直らないか、再試行の上限に達した)。
    #[error("API error {status} ({kind}): {message}")]
    Api {
        status: u16,
        /// API のエラー種別 (`invalid_request_error`・`overloaded_error` など)。
        kind: String,
        message: String,
    },
    /// レート制限 (HTTP 429) で、再試行の上限に達した。
    #[error("rate limited{}", retry_after.map(|d| format!(" (retry after {}s)", d.as_secs())).unwrap_or_default())]
    RateLimited { retry_after: Option<Duration> },
    /// 接続できない・タイムアウトした。
    #[error("network error: {0}")]
    Network(String),
    /// モデルが安全上の理由で応答を断った。
    #[error("the model declined the request{}", category.as_ref().map(|c| format!(" ({c})")).unwrap_or_default())]
    Refused { category: Option<String> },
    /// 応答が `max_tokens` で切れた。
    #[error("the response was cut off at the max_tokens limit")]
    Truncated,
    /// 応答の形式が想定と違う (JSON として読めない、など)。
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

/// 画像とテキストを入力に構造化 JSON を返すモデルの抽象。
pub trait Provider {
    /// `schema` (JSON Schema) に従う JSON を返させる。
    fn complete_json(
        &self,
        request: &ProviderRequest,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, AiError>;

    /// `request` を送るときに、どこへ何が出ていくか。
    ///
    /// エディタは外部へ送る前にこれを利用者に示して確認を取る
    /// (`docs/ai-integration.md#セキュリティとプライバシー`)。既定はローカルで完結する扱い。
    fn disclosure(&self, request: &ProviderRequest) -> Disclosure {
        Disclosure::new(Destination::Local, request)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub system: String,
    pub prompt: String,
    /// PNG バイト列。
    #[serde(skip)]
    pub images: Vec<Vec<u8>>,
}

/// リクエストの送り先。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// 外部に送らない (ローカルモデル・テスト用のプロバイダ)。
    Local,
    /// 外部のサービスに送る。
    External {
        /// サービスの名前 (例: `Anthropic Claude API`)。
        service: String,
        /// 送り先の URL。
        endpoint: String,
        /// 使うモデルの名前。
        model: String,
    },
}

/// 送る内容の要約。利用者に確認を求めるときに示す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disclosure {
    pub destination: Destination,
    /// 送る画像の枚数。
    pub image_count: usize,
    /// 送る画像の合計バイト数 (エンコード前)。
    pub image_bytes: usize,
    /// 送るテキスト (システムプロンプトとプロンプト) の合計バイト数。
    pub text_bytes: usize,
}

impl Disclosure {
    pub fn new(destination: Destination, request: &ProviderRequest) -> Self {
        Self {
            destination,
            image_count: request.images.len(),
            image_bytes: request.images.iter().map(Vec::len).sum(),
            text_bytes: request.system.len() + request.prompt.len(),
        }
    }

    /// 外部のサービスにデータが出ていくか。
    pub fn is_external(&self) -> bool {
        matches!(self.destination, Destination::External { .. })
    }
}

/// AI からの提案。ユーザー承認前はモデルに適用されない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub title: String,
    pub rationale: String,
    pub commands: Vec<Command>,
}

// TODO(T0504): 自動パラメータ・キーフォーム設定 (`propose_rig`)。
