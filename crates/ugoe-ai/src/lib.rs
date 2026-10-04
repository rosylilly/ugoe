//! AI 連携。設計: `docs/ai-integration.md`。
//!
//! 原則:
//! - AI は **[`ugoe_editor_core::Command`] の列を提案するだけ** で、モデルを直接書き換えない。
//!   提案はエディタで差分プレビューされ、ユーザーが採用して初めて適用される。
//! - LLM / 画像モデルの呼び出しは [`Provider`] トレイトの裏に隠し、ネットワーク無しで
//!   テストできるようにする (テストでは [`FakeProvider`] か、録画した応答を再生する
//!   [`transport::ReplayTransport`] を使う)。
//! - 既定のプロバイダは Anthropic の Claude API ([`claude::ClaudeProvider`])。OpenAI 互換の API
//!   ([`openai::OpenAiProvider`]) も使える (ADR 0022)。

mod api_key;
pub mod cancel;
pub mod classify;
pub mod claude;
pub mod depth;
mod fake;
mod grid;
pub mod instructions;
pub mod module_rig;
pub mod normal;
pub mod openai;
pub mod rig;
pub mod transport;
pub mod turn;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use ugoe_editor_core::Command;

pub use api_key::ApiKey;
pub use cancel::CancelToken;
pub use fake::FakeProvider;

/// PNG の署名。プロバイダは送る前に画像が PNG であることを確かめる。
pub(crate) const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// AI の呼び出しと提案のエラー。
///
/// 文言 (`Display`) は英語で、API キーを含まない。待ち時間などの秒は切り上げて示す ([`secs_rounded_up`])。
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// プロバイダ自身の失敗で、他のヴァリアントに当たらないもの
    /// (テスト用の [`FakeProvider`] の応答が尽きた・リクエストを送るスレッドを作れない、など)。
    #[error("provider error: {0}")]
    Provider(String),
    /// AI の答え (値) が原因で提案を組めない (範囲の外・検証を通らない、など)。
    /// リグの提案は答えを変えて問い直す。
    #[error("model returned an invalid proposal: {0}")]
    InvalidProposal(String),
    /// API キーが見つからない・空である。
    #[error("API key is not configured: {0}")]
    MissingApiKey(String),
    /// 入力の誤りで、AI の答えを変えても直らないもの (画像が PNG でない・テンプレートがそのモデルを組めない、など)。
    /// 問い直さない。
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// API がエラーを返した (再試行しても直らないか、再試行の上限に達した)。
    #[error("API error {status} ({kind}): {message}")]
    Api {
        /// HTTP の状態コード。
        status: u16,
        /// API のエラー種別 (`invalid_request_error`・`overloaded_error` など)。
        kind: String,
        /// API が返したエラーの文言 (API キーを取り除き、長いものは切り詰めたもの)。
        message: String,
    },
    /// レート制限 (HTTP 429) で、再試行の上限に達した。
    #[error("rate limited{}", retry_after.map(|d| format!(" (retry after {}s)", secs_rounded_up(d))).unwrap_or_default())]
    RateLimited {
        /// API が示した待ち時間 (`retry-after`)。示されなければ `None`。
        retry_after: Option<Duration>,
    },
    /// 接続できない・タイムアウトした。
    #[error("network error: {0}")]
    Network(String),
    /// モデルが安全上の理由で応答を断った。
    #[error("the model declined the request{}", category.as_ref().map(|c| format!(" ({c})")).unwrap_or_default())]
    Refused {
        /// 断った理由の分類 (API が示したとき。`cyber` など)。
        category: Option<String>,
    },
    /// 応答が `max_tokens` で切れた。
    #[error("the response was cut off at the max_tokens limit")]
    Truncated,
    /// 応答の形式が想定と違う (JSON として読めない、など)。
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    /// 利用者が取り消した ([`CancelToken::cancel`])。再試行も問い直しもしない。
    #[error("the request was cancelled")]
    Cancelled,
    /// 1 回の提案の時間の合計 (再試行と問い直しを含む) が上限 ([`CancelToken::with_time_limit`]) を超えた。
    #[error("the time limit of {}s was exceeded", secs_rounded_up(*limit))]
    TimeLimitExceeded {
        /// 超えた上限。
        limit: Duration,
    },
}

/// 秒に切り上げる。待ち時間を示すときに使う (0.5 秒を「0 秒」と示さない)。
///
/// エディタなどのホストが [`AiError`] の待ち時間を画面に出すときも、これでそろえる。
pub fn secs_rounded_up(d: Duration) -> u64 {
    d.as_secs() + u64::from(d.subsec_nanos() > 0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secs_round_up() {
        assert_eq!(secs_rounded_up(Duration::ZERO), 0);
        assert_eq!(secs_rounded_up(Duration::from_millis(1)), 1);
        assert_eq!(secs_rounded_up(Duration::from_millis(500)), 1);
        assert_eq!(secs_rounded_up(Duration::from_secs(2)), 2);
        assert_eq!(secs_rounded_up(Duration::from_millis(2001)), 3);
    }

    #[test]
    fn waits_are_shown_rounded_up() {
        let e = AiError::RateLimited {
            retry_after: Some(Duration::from_millis(500)),
        };
        assert_eq!(e.to_string(), "rate limited (retry after 1s)");
        let e = AiError::RateLimited {
            retry_after: Some(Duration::from_secs(30)),
        };
        assert_eq!(e.to_string(), "rate limited (retry after 30s)");
        let e = AiError::RateLimited { retry_after: None };
        assert_eq!(e.to_string(), "rate limited");
        let e = AiError::TimeLimitExceeded {
            limit: Duration::from_millis(1500),
        };
        assert_eq!(e.to_string(), "the time limit of 2s was exceeded");
    }
}
