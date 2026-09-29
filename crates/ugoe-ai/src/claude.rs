//! Anthropic の Claude API (Messages API) を呼ぶプロバイダ。
//!
//! - 画像 (PNG) とテキストを送り、構造化出力 (`output_config.format` の JSON Schema) で
//!   スキーマに沿った JSON を受け取る。
//! - HTTP 429・408・409・5xx (529 overloaded を含む)・接続の失敗・タイムアウトは、
//!   指数バックオフで再試行する。`retry-after` ヘッダがあればそれに従う。
//! - API キーはログ・エラー・録画に出さない ([`ApiKey`] の `Debug` は伏せ字で、エラーの
//!   文言からもキーを取り除く)。
//!
//! 設計: `docs/ai-integration.md`。API: <https://docs.claude.com/en/api/messages>。

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::transport::{HttpRequest, HttpResponse, Transport, TransportError, UreqTransport};
use crate::{AiError, Destination, Disclosure, Provider, ProviderRequest};

/// 既定のモデル。
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
/// 既定の API の URL。
pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
/// API キーを読む環境変数。
pub const API_KEY_ENV: &str = "ANTHROPIC_API_KEY";
/// 送る `anthropic-version` ヘッダ。
const ANTHROPIC_VERSION: &str = "2023-06-01";
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// API キー。値は `Debug` やエラーに出ない。
///
/// OS の資格情報の保存先 (キーチェーンなど) から読むのはホスト (エディタ) の役目で、
/// 読んだ値を [`ApiKey::new`] に渡す。
#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(key: impl Into<String>) -> Result<Self, AiError> {
        let key = key.into().trim().to_owned();
        if key.is_empty() {
            return Err(AiError::MissingApiKey("the API key is empty".into()));
        }
        Ok(Self(key))
    }

    /// 環境変数 `ANTHROPIC_API_KEY` から読む。
    pub fn from_env() -> Result<Self, AiError> {
        match std::env::var(API_KEY_ENV) {
            Ok(v) => {
                Self::new(v).map_err(|_| AiError::MissingApiKey(format!("{API_KEY_ENV} is empty")))
            }
            Err(_) => Err(AiError::MissingApiKey(format!("{API_KEY_ENV} is not set"))),
        }
    }

    fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(<redacted>)")
    }
}

/// 考える量 (`output_config.effort`)。省くと API の既定 (`claude-opus-5-5` では `medium`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

/// 再試行の方針。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    /// 最初の試行に加えて再試行する回数。
    pub max_retries: u32,
    /// 最初の待ち時間。再試行ごとに倍にする。
    pub initial_backoff: Duration,
    /// 待ち時間の上限 (`retry-after` がないとき)。
    pub max_backoff: Duration,
    /// `retry-after` がこれより長いときは待たずに諦める。
    pub max_retry_after: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(8),
            max_retry_after: Duration::from_secs(60),
        }
    }
}

impl RetryPolicy {
    /// `attempt` 回目 (0 始まり) の失敗の後に待つ時間。
    fn backoff(&self, attempt: u32) -> Duration {
        let factor = 1u32.checked_shl(attempt).unwrap_or(u32::MAX);
        self.initial_backoff
            .saturating_mul(factor)
            .min(self.max_backoff)
    }
}

/// [`ClaudeProvider`] の設定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeConfig {
    /// モデルの ID。
    pub model: String,
    /// 応答の最大トークン数 (考える分を含む)。
    pub max_tokens: u32,
    /// 考える量。`None` なら API の既定。
    pub effort: Option<Effort>,
    /// API の URL (末尾の `/` なし)。
    pub base_url: String,
    /// 1 回のリクエストの時間の上限。
    pub timeout: Duration,
    pub retry: RetryPolicy,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_owned(),
            max_tokens: 16_000,
            effort: None,
            base_url: DEFAULT_BASE_URL.to_owned(),
            timeout: Duration::from_secs(600),
            retry: RetryPolicy::default(),
        }
    }
}

impl ClaudeConfig {
    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.base_url.trim_end_matches('/'))
    }
}

type Sleeper = Arc<dyn Fn(Duration) + Send + Sync>;

/// Claude API を呼ぶ [`Provider`]。
pub struct ClaudeProvider {
    config: ClaudeConfig,
    api_key: ApiKey,
    transport: Box<dyn Transport>,
    sleep: Sleeper,
}

impl fmt::Debug for ClaudeProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaudeProvider")
            .field("config", &self.config)
            .field("api_key", &self.api_key)
            .finish_non_exhaustive()
    }
}

impl ClaudeProvider {
    /// 実際の API と通信するプロバイダ。
    pub fn new(config: ClaudeConfig, api_key: ApiKey) -> Self {
        let transport = UreqTransport::new(config.timeout);
        Self::with_transport(config, api_key, transport)
    }

    /// 通信を差し替えたプロバイダ (テストでは [`crate::transport::ReplayTransport`] を渡す)。
    pub fn with_transport(
        config: ClaudeConfig,
        api_key: ApiKey,
        transport: impl Transport + 'static,
    ) -> Self {
        Self {
            config,
            api_key,
            transport: Box::new(transport),
            sleep: Arc::new(std::thread::sleep),
        }
    }

    /// 再試行の前に待つ関数を差し替える (テストで実際に待たないため)。
    pub fn with_sleeper(mut self, sleep: impl Fn(Duration) + Send + Sync + 'static) -> Self {
        self.sleep = Arc::new(sleep);
        self
    }

    pub fn config(&self) -> &ClaudeConfig {
        &self.config
    }

    /// Messages API に送る JSON を作る。
    fn build_body(
        &self,
        request: &ProviderRequest,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, AiError> {
        let engine = base64::engine::general_purpose::STANDARD;
        let mut content = Vec::with_capacity(request.images.len() + 1);
        for (i, png) in request.images.iter().enumerate() {
            if !png.starts_with(PNG_SIGNATURE) {
                return Err(AiError::InvalidRequest(format!("image {i} is not a PNG")));
            }
            content.push(json!({
                "type": "image",
                "source": { "type": "base64", "media_type": "image/png", "data": engine.encode(png) },
            }));
        }
        content.push(json!({ "type": "text", "text": request.prompt }));

        let mut output_config = json!({ "format": { "type": "json_schema", "schema": schema } });
        if let Some(effort) = self.config.effort {
            output_config["effort"] = json!(effort);
        }
        let mut body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": [{ "role": "user", "content": content }],
            "output_config": output_config,
        });
        if !request.system.is_empty() {
            body["system"] = json!(request.system);
        }
        Ok(body)
    }

    fn http_request(&self, body: &serde_json::Value) -> HttpRequest {
        HttpRequest {
            method: "POST".into(),
            url: self.config.messages_url(),
            headers: vec![
                ("content-type".into(), "application/json".into()),
                ("anthropic-version".into(), ANTHROPIC_VERSION.into()),
                ("x-api-key".into(), self.api_key.expose().to_owned()),
            ],
            body: body.to_string().into_bytes(),
        }
    }

    /// 再試行しながら送り、成功した応答を返す。
    fn send_with_retry(&self, request: &HttpRequest) -> Result<HttpResponse, AiError> {
        let policy = &self.config.retry;
        let mut attempt = 0;
        loop {
            let can_retry = attempt < policy.max_retries;
            let wait = match self.transport.send(request) {
                Ok(response) if (200..300).contains(&response.status) => return Ok(response),
                Ok(response) => {
                    let retry_after = retry_after(&response);
                    let retryable = is_retryable_status(response.status);
                    if !retryable || !can_retry {
                        return Err(self.status_error(&response, retry_after));
                    }
                    match retry_after {
                        Some(d) if d > policy.max_retry_after => {
                            return Err(self.status_error(&response, retry_after));
                        }
                        Some(d) => d,
                        None => policy.backoff(attempt),
                    }
                }
                Err(e) if e.is_retryable() && can_retry => policy.backoff(attempt),
                Err(e) => return Err(self.transport_error(&e)),
            };
            (self.sleep)(wait);
            attempt += 1;
        }
    }

    fn status_error(&self, response: &HttpResponse, retry_after: Option<Duration>) -> AiError {
        if response.status == 429 {
            return AiError::RateLimited { retry_after };
        }
        let parsed: Option<ErrorBody> = serde_json::from_slice(&response.body).ok();
        let (kind, message) = match parsed {
            Some(b) => (b.error.kind, b.error.message),
            None => (
                "unknown".to_owned(),
                String::from_utf8_lossy(&response.body).into_owned(),
            ),
        };
        AiError::Api {
            status: response.status,
            kind,
            message: self.redact(&message),
        }
    }

    fn transport_error(&self, e: &TransportError) -> AiError {
        AiError::Network(self.redact(&e.to_string()))
    }

    /// 文言に API キーが紛れ込んでいたら伏せる (念のため)。
    fn redact(&self, text: &str) -> String {
        text.replace(self.api_key.expose(), "<redacted>")
    }
}

impl Provider for ClaudeProvider {
    fn complete_json(
        &self,
        request: &ProviderRequest,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, AiError> {
        let body = self.build_body(request, schema)?;
        let response = self.send_with_retry(&self.http_request(&body))?;
        let message: MessageResponse = serde_json::from_slice(&response.body)
            .map_err(|e| AiError::InvalidResponse(format!("not a Messages API response: {e}")))?;
        match message.stop_reason.as_deref() {
            Some("refusal") => {
                return Err(AiError::Refused {
                    category: message.stop_details.and_then(|d| d.category),
                });
            }
            Some("max_tokens") => return Err(AiError::Truncated),
            _ => {}
        }
        // 考えた内容 (`thinking`) などは読み飛ばし、テキストのブロックだけをつなぐ
        let text: String = message
            .content
            .iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text.as_deref())
            .collect();
        if text.trim().is_empty() {
            return Err(AiError::InvalidResponse("the response has no text".into()));
        }
        serde_json::from_str(&text)
            .map_err(|e| AiError::InvalidResponse(format!("the response is not JSON: {e}")))
    }

    fn disclosure(&self, request: &ProviderRequest) -> Disclosure {
        Disclosure::new(
            Destination::External {
                service: "Anthropic Claude API".into(),
                endpoint: self.config.messages_url(),
                model: self.config.model.clone(),
            },
            request,
        )
    }
}

/// 再試行してよい状態コード (408 Request Timeout・409 Conflict・429・5xx)。
fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 409 | 429) || status >= 500
}

/// `retry-after` ヘッダ (秒)。
fn retry_after(response: &HttpResponse) -> Option<Duration> {
    let secs: f64 = response.header("retry-after")?.trim().parse().ok()?;
    (secs.is_finite() && secs >= 0.0).then(|| Duration::from_secs_f64(secs))
}

#[derive(Deserialize)]
struct MessageResponse {
    #[serde(default)]
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    stop_details: Option<StopDetails>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct StopDetails {
    category: Option<String>,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
    #[serde(rename = "type")]
    kind: String,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_up_to_the_cap() {
        let p = RetryPolicy::default();
        assert_eq!(p.backoff(0), Duration::from_millis(500));
        assert_eq!(p.backoff(1), Duration::from_secs(1));
        assert_eq!(p.backoff(10), Duration::from_secs(8));
        assert_eq!(p.backoff(40), Duration::from_secs(8));
    }

    #[test]
    fn retryable_statuses() {
        for s in [408, 409, 429, 500, 503, 529] {
            assert!(is_retryable_status(s), "{s}");
        }
        for s in [400, 401, 403, 404, 413] {
            assert!(!is_retryable_status(s), "{s}");
        }
    }

    #[test]
    fn api_key_rejects_empty_and_hides_value() {
        assert!(matches!(ApiKey::new("  "), Err(AiError::MissingApiKey(_))));
        let key = ApiKey::new("sk-ant-hidden").unwrap();
        assert_eq!(format!("{key:?}"), "ApiKey(<redacted>)");
    }

    #[test]
    fn messages_url_ignores_trailing_slash() {
        let c = ClaudeConfig {
            base_url: "http://localhost:1/".into(),
            ..Default::default()
        };
        assert_eq!(c.messages_url(), "http://localhost:1/v1/messages");
    }
}
