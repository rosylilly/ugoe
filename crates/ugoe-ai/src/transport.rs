//! HTTP の送受信を差し替えるための層。
//!
//! 実際の通信は [`UreqTransport`]、テストでは録画 ([`Cassette`]) を再生する [`ReplayTransport`]
//! を使う。[`RecordingTransport`] は実際の通信を包んで録画を作る。
//!
//! 録画には API キーを含むリクエストヘッダを残さず、送った画像のデータも省く
//! (`docs/ai-integration.md#セキュリティとプライバシー`)。

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// 値をログや録画に出してはならないヘッダ (小文字)。
const SENSITIVE_HEADERS: &[&str] = &["x-api-key", "authorization"];
/// 録画に残す応答ヘッダ (小文字)。再試行の判断と調査に使うものだけ。
const RECORDED_RESPONSE_HEADERS: &[&str] =
    &["content-type", "request-id", "retry-after", "x-request-id"];

/// 送る HTTP リクエスト。
#[derive(Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    /// ヘッダ名は小文字で持つ。
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    /// 秘密の値を伏せたヘッダ。
    pub fn redacted_headers(&self) -> Vec<(String, String)> {
        self.headers
            .iter()
            .map(|(k, v)| {
                let v = if is_sensitive(k) {
                    "<redacted>".to_owned()
                } else {
                    v.clone()
                };
                (k.clone(), v)
            })
            .collect()
    }
}

impl fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &self.redacted_headers())
            .field("body_len", &self.body.len())
            .finish()
    }
}

/// 受け取った HTTP 応答。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    /// ヘッダ名は小文字で持つ。
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// ヘッダの値 (名前は小文字で渡す)。
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// 通信の失敗の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportErrorKind {
    /// タイムアウト。
    Timeout,
    /// 接続できない (名前解決・接続の失敗・途中で切れた)。
    Connect,
    /// それ以外 (録画が尽きた、など)。再試行しない。
    Other,
}

/// 応答を受け取る前の失敗。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[error("{kind:?}: {message}")]
pub struct TransportError {
    pub kind: TransportErrorKind,
    pub message: String,
}

impl TransportError {
    pub fn new(kind: TransportErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// 再試行してよい失敗か。
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.kind,
            TransportErrorKind::Timeout | TransportErrorKind::Connect
        )
    }
}

/// HTTP リクエストを 1 回送る。再試行は呼び出し側 (プロバイダ) が行う。
pub trait Transport: Send + Sync {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError>;
}

/// 共有したまま渡せるように (テストで受け取ったリクエストを後から見る、など)。
impl<T: Transport + ?Sized> Transport for std::sync::Arc<T> {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        (**self).send(request)
    }
}

/// ureq による実際の通信。
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    /// `timeout` はリクエスト全体 (接続から応答の読み終わりまで) の上限。
    pub fn new(timeout: Duration) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .timeout_connect(Some(timeout.min(Duration::from_secs(30))))
            // 4xx/5xx も応答として受け取り、状態コードで判断する
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

impl fmt::Debug for UreqTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UreqTransport").finish_non_exhaustive()
    }
}

impl Transport for UreqTransport {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        let mut builder = match request.method.as_str() {
            "POST" => self.agent.post(&request.url),
            other => {
                return Err(TransportError::new(
                    TransportErrorKind::Other,
                    format!("unsupported method {other}"),
                ));
            }
        };
        for (k, v) in &request.headers {
            builder = builder.header(k, v);
        }
        let mut response = builder.send(&request.body[..]).map_err(map_ureq_error)?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                Some((k.as_str().to_ascii_lowercase(), v.to_str().ok()?.to_owned()))
            })
            .collect();
        let body = response.body_mut().read_to_vec().map_err(map_ureq_error)?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

fn map_ureq_error(e: ureq::Error) -> TransportError {
    let kind = match &e {
        ureq::Error::Timeout(_) => TransportErrorKind::Timeout,
        ureq::Error::Io(_) | ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => {
            TransportErrorKind::Connect
        }
        _ => TransportErrorKind::Other,
    };
    TransportError::new(kind, e.to_string())
}

/// 録画した 1 往復。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interaction {
    pub request: RecordedRequest,
    /// 応答か、応答を受け取る前の失敗のどちらか。
    #[serde(flatten)]
    pub outcome: RecordedOutcome,
}

/// 録画したリクエスト。ヘッダは残さない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedRequest {
    pub method: String,
    pub url: String,
    /// 送った JSON (画像のデータは省いたもの)。
    /// 再生のときに書いてあれば、送ったものと一致するかを確かめる。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

impl RecordedRequest {
    /// 送ったリクエストから作る (画像のデータを省く)。
    pub fn from_request(request: &HttpRequest) -> Self {
        let body = serde_json::from_slice(&request.body).ok().map(|mut v| {
            redact_binary(&mut v);
            v
        });
        Self {
            method: request.method.clone(),
            url: request.url.clone(),
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordedOutcome {
    Response(RecordedResponse),
    Error(TransportError),
}

/// 録画した応答。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedResponse {
    pub status: u16,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    /// JSON として読めた本文。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    /// JSON でない本文。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_text: Option<String>,
}

impl RecordedResponse {
    fn from_response(response: &HttpResponse) -> Self {
        let headers = response
            .headers
            .iter()
            .filter(|(k, _)| RECORDED_RESPONSE_HEADERS.contains(&k.as_str()))
            .cloned()
            .collect();
        let (body, body_text) = match serde_json::from_slice(&response.body) {
            Ok(v) => (Some(v), None),
            Err(_) => (
                None,
                Some(String::from_utf8_lossy(&response.body).into_owned()),
            ),
        };
        Self {
            status: response.status,
            headers,
            body,
            body_text,
        }
    }

    fn to_response(&self) -> HttpResponse {
        let body = match (&self.body, &self.body_text) {
            (Some(v), _) => v.to_string().into_bytes(),
            (None, Some(t)) => t.clone().into_bytes(),
            (None, None) => Vec::new(),
        };
        HttpResponse {
            status: self.status,
            headers: self
                .headers
                .iter()
                .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
                .collect(),
            body,
        }
    }
}

/// 録画 (往復の列)。JSON ファイルとして保存する。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Cassette {
    pub interactions: Vec<Interaction>,
}

impl Cassette {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Self::from_json(&text).map_err(std::io::Error::other)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut text = self.to_json().map_err(std::io::Error::other)?;
        text.push('\n');
        std::fs::write(path, text)
    }
}

/// 録画を順に返す。ネットワークを使わない。
#[derive(Debug)]
pub struct ReplayTransport {
    queue: Mutex<VecDeque<Interaction>>,
    received: Mutex<Vec<ReceivedRequest>>,
}

/// [`ReplayTransport`] が受け取ったリクエスト (秘密のヘッダは伏せてある)。
#[derive(Debug, Clone, PartialEq)]
pub struct ReceivedRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    /// 画像のデータは省いてある。
    pub body: Option<serde_json::Value>,
}

impl ReplayTransport {
    pub fn new(cassette: Cassette) -> Self {
        Self {
            queue: Mutex::new(cassette.interactions.into()),
            received: Mutex::default(),
        }
    }

    /// これまでに受け取ったリクエスト。
    pub fn received(&self) -> Vec<ReceivedRequest> {
        lock(&self.received).clone()
    }

    /// まだ返していない往復の数。
    pub fn remaining(&self) -> usize {
        lock(&self.queue).len()
    }
}

impl Transport for ReplayTransport {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        let recorded = RecordedRequest::from_request(request);
        lock(&self.received).push(ReceivedRequest {
            method: request.method.clone(),
            url: request.url.clone(),
            headers: request.redacted_headers(),
            body: recorded.body.clone(),
        });
        let Some(next) = lock(&self.queue).pop_front() else {
            return Err(TransportError::new(
                TransportErrorKind::Other,
                "cassette exhausted",
            ));
        };
        if next.request.method != recorded.method || next.request.url != recorded.url {
            return Err(TransportError::new(
                TransportErrorKind::Other,
                format!(
                    "request mismatch: expected {} {}, got {} {}",
                    next.request.method, next.request.url, recorded.method, recorded.url
                ),
            ));
        }
        if let Some(expected) = &next.request.body
            && recorded.body.as_ref() != Some(expected)
        {
            return Err(TransportError::new(
                TransportErrorKind::Other,
                "request body does not match the cassette",
            ));
        }
        match next.outcome {
            RecordedOutcome::Response(r) => Ok(r.to_response()),
            RecordedOutcome::Error(e) => Err(e),
        }
    }
}

/// 別の [`Transport`] を包み、往復を録画する。
#[derive(Debug)]
pub struct RecordingTransport<T> {
    inner: T,
    cassette: Mutex<Cassette>,
}

impl<T: Transport> RecordingTransport<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            cassette: Mutex::default(),
        }
    }

    /// これまでの録画。
    pub fn cassette(&self) -> Cassette {
        lock(&self.cassette).clone()
    }
}

impl<T: Transport> Transport for RecordingTransport<T> {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        let result = self.inner.send(request);
        let outcome = match &result {
            Ok(r) => RecordedOutcome::Response(RecordedResponse::from_response(r)),
            Err(e) => RecordedOutcome::Error(e.clone()),
        };
        lock(&self.cassette).interactions.push(Interaction {
            request: RecordedRequest::from_request(request),
            outcome,
        });
        result
    }
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

/// [`send_with_retry`] が諦めた理由。
pub(crate) enum SendFailure {
    /// 成功でない応答 (再試行しない状態・再試行の上限・`retry-after` が長すぎる)。
    Status {
        response: HttpResponse,
        retry_after: Option<Duration>,
    },
    /// 応答を受け取る前の失敗。
    Transport(TransportError),
}

/// 再試行しながら送り、成功 (2xx) の応答を返す。プロバイダで共通の再試行の規則:
/// `retryable` が真の応答と、タイムアウト・接続の失敗を指数バックオフで再試行する。
/// `retry-after` ヘッダ (秒) があればそれだけ待ち、`max_retry_after` より長ければ諦める。
pub(crate) fn send_with_retry(
    transport: &dyn Transport,
    policy: &RetryPolicy,
    sleep: &dyn Fn(Duration),
    request: &HttpRequest,
    retryable: impl Fn(&HttpResponse) -> bool,
) -> Result<HttpResponse, SendFailure> {
    let mut attempt = 0;
    loop {
        let can_retry = attempt < policy.max_retries;
        let wait = match transport.send(request) {
            Ok(response) if (200..300).contains(&response.status) => return Ok(response),
            Ok(response) => {
                let retry_after = retry_after(&response);
                if !retryable(&response) || !can_retry {
                    return Err(SendFailure::Status {
                        response,
                        retry_after,
                    });
                }
                match retry_after {
                    Some(d) if d > policy.max_retry_after => {
                        return Err(SendFailure::Status {
                            response,
                            retry_after,
                        });
                    }
                    Some(d) => d,
                    None => policy.backoff(attempt),
                }
            }
            Err(e) if e.is_retryable() && can_retry => policy.backoff(attempt),
            Err(e) => return Err(SendFailure::Transport(e)),
        };
        sleep(wait);
        attempt += 1;
    }
}

/// 再試行してよい状態コード (408 Request Timeout・409 Conflict・429・5xx)。
pub(crate) fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 409 | 429) || status >= 500
}

/// `retry-after` ヘッダ (秒)。
fn retry_after(response: &HttpResponse) -> Option<Duration> {
    let secs: f64 = response.header("retry-after")?.trim().parse().ok()?;
    (secs.is_finite() && secs >= 0.0).then(|| Duration::from_secs_f64(secs))
}

/// URL のホスト名 (小文字。IPv6 は角括弧を除く)。スキームがなくてもよい。
pub fn url_host(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // `user:pass@host` の利用者の部分は除く
    let host = authority.rsplit('@').next().unwrap_or_default();
    let host = match host.strip_prefix('[') {
        // IPv6 (`[::1]:8765`)
        Some(v6) => v6.split(']').next().unwrap_or_default(),
        None => host.split(':').next().unwrap_or_default(),
    };
    host.to_ascii_lowercase()
}

/// URL の送り先がこの計算機 (ループバック: `localhost`・`127.0.0.0/8`・`::1`) か。
/// ループバックへの送信は外部に出ない扱い ([`crate::Destination::Local`]) にする。
pub fn is_loopback_url(url: &str) -> bool {
    let host = url_host(url);
    if host == "localhost" || host == "::1" {
        return true;
    }
    // `127.x.y.z` (数字 4 つ)。`127.example.com` のような名前は含めない
    let octets: Vec<&str> = host.split('.').collect();
    octets.len() == 4 && octets[0] == "127" && octets.iter().all(|o| o.parse::<u8>().is_ok())
}

fn is_sensitive(header: &str) -> bool {
    SENSITIVE_HEADERS
        .iter()
        .any(|s| header.eq_ignore_ascii_case(s))
}

/// `{"type": "base64", "data": "..."}` の `data` と data URL の本体を、長さだけを書いた文字列に置き換える。
fn redact_binary(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("type").and_then(|t| t.as_str()) == Some("base64")
                && let Some(serde_json::Value::String(data)) = map.get_mut("data")
            {
                *data = format!("<{} base64 chars omitted>", data.len());
            }
            for v in map.values_mut() {
                redact_binary(v);
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(redact_binary),
        // data URL (`data:image/png;base64,...`。OpenAI 互換の API の画像)
        serde_json::Value::String(text) => {
            if let Some((head, data)) = text.split_once(";base64,")
                && head.starts_with("data:")
            {
                *text = format!("{head};base64,<{} base64 chars omitted>", data.len());
            }
        }
        _ => {}
    }
}

/// 毒化したロックもそのまま使う (中身は単純なキューで、壊れた状態にならない)。
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request(body: serde_json::Value) -> HttpRequest {
        HttpRequest {
            method: "POST".into(),
            url: "https://example.test/v1/messages".into(),
            headers: vec![("x-api-key".into(), "sk-secret".into())],
            body: body.to_string().into_bytes(),
        }
    }

    #[test]
    fn debug_hides_the_api_key() {
        let r = request(json!({}));
        assert!(!format!("{r:?}").contains("sk-secret"));
    }

    #[test]
    fn recording_omits_headers_and_image_data() {
        let mut replay = Cassette::default();
        replay.interactions.push(Interaction {
            request: RecordedRequest {
                method: "POST".into(),
                url: "https://example.test/v1/messages".into(),
                body: None,
            },
            outcome: RecordedOutcome::Response(RecordedResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: Some(json!({"ok": true})),
                body_text: None,
            }),
        });
        let rec = RecordingTransport::new(ReplayTransport::new(replay));
        let body = json!({"content": [{"type": "image", "source": {"type": "base64", "data": "QUJDRA=="}}]});
        rec.send(&request(body)).unwrap();
        let json = rec.cassette().to_json().unwrap();
        assert!(!json.contains("sk-secret"));
        assert!(!json.contains("QUJDRA=="));
        assert!(json.contains("<8 base64 chars omitted>"));
    }

    #[test]
    fn recording_omits_data_urls() {
        let mut v =
            json!({"image_url": {"url": "data:image/png;base64,QUJDRA=="}, "text": "data: plain"});
        redact_binary(&mut v);
        assert_eq!(
            v,
            json!({"image_url": {"url": "data:image/png;base64,<8 base64 chars omitted>"}, "text": "data: plain"})
        );
    }

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
    fn loopback_urls() {
        for url in [
            "http://127.0.0.1:8765/depth",
            "http://localhost/depth",
            "http://LOCALHOST:11434/v1",
            "http://[::1]:80/",
            "http://127.1.2.3/",
            "http://user:pw@localhost:1234/v1",
            "localhost:1234",
        ] {
            assert!(is_loopback_url(url), "{url}");
        }
        for url in [
            "https://depth.example.com/v1",
            "https://localhost.example.com/",
            "https://127.0.0.1.example.com/",
            "https://example.com/?next=http://localhost",
            "https://localhost@example.com/",
            "http://192.168.0.10:11434/v1",
            "http://0.0.0.0:11434/v1",
        ] {
            assert!(!is_loopback_url(url), "{url}");
        }
    }

    #[test]
    fn replay_reports_exhaustion_and_mismatch() {
        let t = ReplayTransport::new(Cassette::default());
        let e = t.send(&request(json!({}))).unwrap_err();
        assert_eq!(e.kind, TransportErrorKind::Other);
        assert!(!e.is_retryable());

        let cassette = Cassette::from_json(
            r#"{"interactions":[{"request":{"method":"POST","url":"https://other.test/"},"response":{"status":200}}]}"#,
        )
        .unwrap();
        let t = ReplayTransport::new(cassette);
        assert!(
            t.send(&request(json!({})))
                .unwrap_err()
                .message
                .contains("mismatch")
        );
    }

    #[test]
    fn cassette_round_trips_errors() {
        let json = r#"{"interactions":[{"request":{"method":"POST","url":"u"},"error":{"kind":"timeout","message":"timed out"}}]}"#;
        let c = Cassette::from_json(json).unwrap();
        assert_eq!(
            c.interactions[0].outcome,
            RecordedOutcome::Error(TransportError::new(
                TransportErrorKind::Timeout,
                "timed out"
            ))
        );
        assert_eq!(Cassette::from_json(&c.to_json().unwrap()).unwrap(), c);
    }
}
