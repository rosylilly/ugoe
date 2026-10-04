//! OpenAI 互換の API (Chat Completions) を呼ぶプロバイダ。ADR 0022。
//!
//! これ 1 つで OpenAI・OpenRouter・Gemini の OpenAI 互換の窓口・手元の Ollama / LM Studio / vLLM などを扱う。
//!
//! - 画像 (PNG) は data URL (`data:image/png;base64,...`) の `image_url` として、テキストと一緒に送る。
//! - 答えの形は [`StructuredOutput`] で選ぶ。既定は構造化出力 (`response_format` の `json_schema`、
//!   `strict: true`)。対応しないサーバでは `json_object` (JSON であることだけを保証) か、プロンプトに
//!   スキーマを書くだけにして、受け取った JSON をスキーマで検証し、通らなければ前の答えと理由を添えて
//!   問い直す ([`OpenAiConfig::json_retries`])。
//! - HTTP 429・408・409・5xx・接続の失敗・タイムアウトは指数バックオフで再試行し、`retry-after` に従う。
//!   ただし 429 でも残高・利用上限の不足 (`insufficient_quota` など) は再試行しても直らないので諦める。
//! - 送り先がループバック (`localhost`・`127.0.0.1`・`::1`) なら外部に送らない扱い ([`Destination::Local`])。
//! - API キーは省略できる (手元のサーバ)。キーはログ・エラー・録画に出さない。
//!
//! 設計: `docs/ai-integration.md#openai-互換の-api`。
//! API: <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create>、
//! <https://developers.openai.com/api/docs/guides/structured-outputs>、
//! <https://developers.openai.com/api/docs/guides/error-codes>。

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::claude::ApiKey;
use crate::transport::{
    HttpRequest, HttpResponse, RetryPolicy, SendFailure, Transport, UreqTransport, error_excerpt,
    is_loopback_url, is_retryable_status, send_with_retry, url_host,
};
use crate::{AiError, Destination, Disclosure, PNG_SIGNATURE, Provider, ProviderRequest};

/// OpenAI の API の URL (`/chat/completions` の手前まで)。
pub const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
/// OpenAI の API キーを読む環境変数 (送り先が OpenAI のときだけ使う)。
pub const API_KEY_ENV: &str = "OPENAI_API_KEY";
/// 構造化出力のスキーマの名前 (`response_format.json_schema.name`)。
const SCHEMA_NAME: &str = "ugoe_response";

/// 答えを JSON にさせる方法。サーバの対応に合わせて選ぶ。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredOutput {
    /// 構造化出力 (`response_format: {type: "json_schema", json_schema: {..., strict: true}}`)。
    /// OpenAI・OpenRouter (対応するモデル)・Gemini・Ollama・LM Studio・vLLM が対応する。
    #[default]
    JsonSchema,
    /// JSON モード (`response_format: {type: "json_object"}`)。JSON であることだけを保証し、
    /// スキーマはプロンプトに書く。
    JsonObject,
    /// `response_format` を送らず、スキーマをプロンプトに書くだけ。どのサーバでも使える。
    Prompt,
}

impl StructuredOutput {
    /// 答えが検証を通らなかったときに問い直す回数の既定。構造化出力はスキーマどおりに答えるので問い直さない。
    pub const fn default_json_retries(self) -> u32 {
        match self {
            Self::JsonSchema => 0,
            Self::JsonObject | Self::Prompt => 2,
        }
    }
}

/// 応答のトークン数の上限を送るフィールド。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaxTokensField {
    /// `max_completion_tokens` (OpenAI の今の名前。推論モデルはこちらしか受け付けない)。
    #[default]
    MaxCompletionTokens,
    /// `max_tokens` (古い名前。Ollama・Gemini の互換の窓口などはこちら)。
    MaxTokens,
}

/// [`OpenAiProvider`] の設定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiConfig {
    /// API の URL。`/chat/completions` の手前まで (例: `https://api.openai.com/v1`・
    /// `http://localhost:11434/v1`・`https://generativelanguage.googleapis.com/v1beta/openai`)。
    pub base_url: String,
    /// モデルの ID。
    pub model: String,
    /// 利用者に示すサービスの名前 ([`service_name`] が送り先から決める)。
    pub service: String,
    pub structured_output: StructuredOutput,
    /// 応答のトークン数の上限。`None` なら送らない (サーバの既定)。
    pub max_tokens: Option<u32>,
    pub max_tokens_field: MaxTokensField,
    /// 答えが JSON として読めないか、スキーマの検証を通らなかったときに問い直す回数。
    pub json_retries: u32,
    /// 1 回のリクエストの時間の上限。
    pub timeout: Duration,
    pub retry: RetryPolicy,
}

impl OpenAiConfig {
    /// 送り先とモデル。答えの形は構造化出力 ([`StructuredOutput::JsonSchema`])。
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        let base_url = base_url.into();
        let structured_output = StructuredOutput::default();
        Self {
            service: service_name(&base_url),
            base_url,
            model: model.into(),
            structured_output,
            max_tokens: None,
            max_tokens_field: MaxTokensField::default(),
            json_retries: structured_output.default_json_retries(),
            timeout: Duration::from_secs(600),
            retry: RetryPolicy::default(),
        }
    }

    /// 答えの形を変える。問い直す回数もその形の既定にする。
    #[must_use]
    pub fn with_structured_output(mut self, mode: StructuredOutput) -> Self {
        self.structured_output = mode;
        self.json_retries = mode.default_json_retries();
        self
    }

    /// 送る URL (`<base_url>/chat/completions`)。
    pub fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    /// 送り先がこの計算機 (ループバック) か。
    pub fn is_local(&self) -> bool {
        is_loopback_url(&self.base_url)
    }
}

/// 送り先の URL から、利用者に示すサービスの名前を決める。
pub fn service_name(base_url: &str) -> String {
    let host = url_host(base_url);
    match host.as_str() {
        "api.openai.com" => "OpenAI API".into(),
        "openrouter.ai" => "OpenRouter".into(),
        "generativelanguage.googleapis.com" => "Google Gemini API".into(),
        _ if is_loopback_url(base_url) => "Local OpenAI-compatible server".into(),
        "" => "OpenAI-compatible API".into(),
        _ => format!("OpenAI-compatible API ({host})"),
    }
}

type Sleeper = Arc<dyn Fn(Duration) + Send + Sync>;

/// OpenAI 互換の API を呼ぶ [`Provider`]。
pub struct OpenAiProvider {
    config: OpenAiConfig,
    api_key: Option<ApiKey>,
    transport: Box<dyn Transport>,
    sleep: Sleeper,
}

impl fmt::Debug for OpenAiProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiProvider")
            .field("config", &self.config)
            .field("api_key", &self.api_key)
            .finish_non_exhaustive()
    }
}

/// 問い直すときに添える、前の答えと検証を通らなかった理由。
struct Rejected {
    answer: String,
    error: String,
}

impl OpenAiProvider {
    /// 実際の API と通信するプロバイダ。`api_key` は手元のサーバなら `None` でよい。
    pub fn new(config: OpenAiConfig, api_key: Option<ApiKey>) -> Self {
        let transport = UreqTransport::new(config.timeout);
        Self::with_transport(config, api_key, transport)
    }

    /// 通信を差し替えたプロバイダ (テストでは [`crate::transport::ReplayTransport`] を渡す)。
    pub fn with_transport(
        config: OpenAiConfig,
        api_key: Option<ApiKey>,
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
    #[must_use]
    pub fn with_sleeper(mut self, sleep: impl Fn(Duration) + Send + Sync + 'static) -> Self {
        self.sleep = Arc::new(sleep);
        self
    }

    pub fn config(&self) -> &OpenAiConfig {
        &self.config
    }

    /// Chat Completions に送る JSON を作る。`rejected` は前の答え (問い直すとき)。
    fn build_body(
        &self,
        request: &ProviderRequest,
        schema: &Value,
        rejected: &[Rejected],
    ) -> Result<Value, AiError> {
        let engine = base64::engine::general_purpose::STANDARD;
        let mut content = Vec::with_capacity(request.images.len() + 1);
        for (i, png) in request.images.iter().enumerate() {
            if !png.starts_with(PNG_SIGNATURE) {
                return Err(AiError::InvalidRequest(format!("image {i} is not a PNG")));
            }
            content.push(json!({
                "type": "image_url",
                "image_url": { "url": format!("data:image/png;base64,{}", engine.encode(png)) },
            }));
        }
        let mut prompt = request.prompt.clone();
        if self.config.structured_output != StructuredOutput::JsonSchema {
            prompt.push_str(&schema_instructions(schema));
        }
        content.push(json!({ "type": "text", "text": prompt }));

        let mut messages = Vec::with_capacity(2 + 2 * rejected.len());
        if !request.system.is_empty() {
            messages.push(json!({ "role": "system", "content": request.system }));
        }
        messages.push(json!({ "role": "user", "content": content }));
        for r in rejected {
            messages.push(json!({ "role": "assistant", "content": r.answer }));
            messages.push(json!({
                "role": "user",
                "content": format!(
                    "That answer was rejected: {}\nReply again with only a JSON object that matches the schema.",
                    r.error
                ),
            }));
        }

        let mut body = json!({ "model": self.config.model, "messages": messages });
        match self.config.structured_output {
            StructuredOutput::JsonSchema => {
                body["response_format"] = json!({
                    "type": "json_schema",
                    "json_schema": { "name": SCHEMA_NAME, "schema": schema, "strict": true },
                });
            }
            StructuredOutput::JsonObject => {
                body["response_format"] = json!({ "type": "json_object" });
            }
            StructuredOutput::Prompt => {}
        }
        if let Some(max) = self.config.max_tokens {
            let field = match self.config.max_tokens_field {
                MaxTokensField::MaxCompletionTokens => "max_completion_tokens",
                MaxTokensField::MaxTokens => "max_tokens",
            };
            body[field] = json!(max);
        }
        Ok(body)
    }

    fn http_request(&self, body: &Value) -> HttpRequest {
        let mut headers = vec![("content-type".into(), "application/json".into())];
        if let Some(key) = &self.api_key {
            headers.push(("authorization".into(), format!("Bearer {}", key.expose())));
        }
        HttpRequest {
            method: "POST".into(),
            url: self.config.chat_url(),
            headers,
            body: body.to_string().into_bytes(),
        }
    }

    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, AiError> {
        send_with_retry(
            self.transport.as_ref(),
            &self.config.retry,
            self.sleep.as_ref(),
            request,
            |r| is_retryable_status(r.status) && !is_quota_error(r),
        )
        .map_err(|failure| match failure {
            SendFailure::Status {
                response,
                retry_after,
            } => self.status_error(&response, retry_after),
            SendFailure::Transport(e) => AiError::Network(self.redact(&e.to_string())),
        })
    }

    fn status_error(&self, response: &HttpResponse, retry_after: Option<Duration>) -> AiError {
        if response.status == 429 && !is_quota_error(response) {
            return AiError::RateLimited { retry_after };
        }
        let (kind, message) = error_detail(response);
        AiError::Api {
            status: response.status,
            kind,
            message: self.redact(&message),
        }
    }

    /// 文言に API キーが紛れ込んでいたら伏せる (念のため)。
    fn redact(&self, text: &str) -> String {
        match &self.api_key {
            Some(key) => text.replace(key.expose(), "<redacted>"),
            None => text.to_owned(),
        }
    }
}

impl Provider for OpenAiProvider {
    fn complete_json(&self, request: &ProviderRequest, schema: &Value) -> Result<Value, AiError> {
        let mut rejected = Vec::new();
        loop {
            let body = self.build_body(request, schema, &rejected)?;
            let response = self.send(&self.http_request(&body))?;
            let text = answer_text(&response)?;
            let error = match parse_json(&text) {
                Ok(value) => match check_schema(&value, schema, "$") {
                    Ok(()) => return Ok(value),
                    Err(e) => format!("the answer does not match the schema: {e}"),
                },
                Err(e) => e,
            };
            if rejected.len() >= self.config.json_retries as usize {
                return Err(AiError::InvalidResponse(error));
            }
            rejected.push(Rejected {
                answer: text,
                error,
            });
        }
    }

    fn disclosure(&self, request: &ProviderRequest) -> Disclosure {
        if self.config.is_local() {
            return Disclosure::new(Destination::Local, request);
        }
        Disclosure::new(
            Destination::External {
                service: self.config.service.clone(),
                endpoint: self.config.chat_url(),
                model: self.config.model.clone(),
            },
            request,
        )
    }
}

/// スキーマをプロンプトに書く (構造化出力を使わないとき)。JSON モードは「JSON」という語を要る。
fn schema_instructions(schema: &Value) -> String {
    format!(
        "\n\nReply with only a JSON object (no code fences, no other text) that matches this JSON Schema:\n{schema}"
    )
}

/// 残高・利用上限の不足 (429 でも再試行しても直らない)。
/// <https://developers.openai.com/api/docs/guides/error-codes>
fn is_quota_error(response: &HttpResponse) -> bool {
    if response.status != 429 {
        return false;
    }
    let Ok(body) = serde_json::from_slice::<Value>(&response.body) else {
        return false;
    };
    let error = &body["error"];
    [&error["code"], &error["type"]]
        .iter()
        .filter_map(|v| v.as_str())
        .any(|s| {
            s == "insufficient_quota"
                || s == "credit_balance_exhausted"
                || s.ends_with("_spend_limit_exceeded")
                || s.ends_with("_usage_limit_exceeded")
        })
}

/// エラーの応答の種別と文言。OpenAI の `{"error": {"message", "type", "code"}}` のほか、
/// `{"error": "..."}` (Ollama の一部) や `{"message": "..."}` (vLLM) も読む。
fn error_detail(response: &HttpResponse) -> (String, String) {
    let body: Value = serde_json::from_slice(&response.body).unwrap_or(Value::Null);
    let error = &body["error"];
    let kind = [&error["code"], &error["type"], &body["type"]]
        .iter()
        .find_map(|v| v.as_str())
        .unwrap_or("unknown")
        .to_owned();
    let message = [&error["message"], error, &body["message"]]
        .iter()
        .find_map(|v| v.as_str())
        .map_or_else(|| error_excerpt(&response.body), str::to_owned);
    (kind, message)
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Option<ChatMessage>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChatMessage {
    /// 文字列か、`{"type": "text", "text"}` の列 (サーバによる)。
    content: Option<Value>,
    refusal: Option<String>,
}

/// 応答の最初の選択肢のテキスト。断り・切れた応答はエラーにする。
fn answer_text(response: &HttpResponse) -> Result<String, AiError> {
    let parsed: ChatResponse = serde_json::from_slice(&response.body)
        .map_err(|e| AiError::InvalidResponse(format!("not a Chat Completions response: {e}")))?;
    let choice = parsed
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| AiError::InvalidResponse("the response has no choices".into()))?;
    let message = choice.message;
    if message.as_ref().and_then(|m| m.refusal.as_ref()).is_some() {
        return Err(AiError::Refused { category: None });
    }
    match choice.finish_reason.as_deref() {
        Some("length") => return Err(AiError::Truncated),
        Some("content_filter") => {
            return Err(AiError::Refused {
                category: Some("content_filter".into()),
            });
        }
        _ => {}
    }
    let text = match message.and_then(|m| m.content) {
        Some(Value::String(s)) => s,
        Some(Value::Array(parts)) => parts
            .iter()
            .filter(|p| p["type"] == "text")
            .filter_map(|p| p["text"].as_str())
            .collect(),
        _ => String::new(),
    };
    if text.trim().is_empty() {
        return Err(AiError::InvalidResponse("the response has no text".into()));
    }
    Ok(text)
}

/// 答えのテキストから JSON を読む。構造化出力に対応しないモデルは、考えた内容 (`<think>…</think>`) や
/// コードフェンス・前置きを付けることがあるので、それを除いて最初の `{` から最後の `}` までを読む。
fn parse_json(text: &str) -> Result<Value, String> {
    let mut text = text.trim();
    if let Some(end) = text.find("</think>")
        && text.starts_with("<think>")
    {
        text = text[end + "</think>".len()..].trim();
    }
    if let Ok(value) = serde_json::from_str(text) {
        return Ok(value);
    }
    let candidate = match (text.find('{'), text.rfind('}')) {
        (Some(start), Some(end)) if start < end => &text[start..=end],
        _ => return Err("the answer is not JSON".into()),
    };
    serde_json::from_str(candidate).map_err(|e| format!("the answer is not JSON: {e}"))
}

/// `value` が `schema` に沿うか。ugoe-ai のスキーマが使う範囲 (`type`・`enum`・`properties`・
/// `required`・`additionalProperties: false`・`items`・`anyOf`) だけを見る。
fn check_schema(value: &Value, schema: &Value, path: &str) -> Result<(), String> {
    if let Some(options) = schema["anyOf"].as_array()
        && !options.iter().any(|s| check_schema(value, s, path).is_ok())
    {
        return Err(format!("{path} matches none of the allowed shapes"));
    }
    match &schema["type"] {
        Value::String(t) => check_type(value, t, path)?,
        Value::Array(types)
            if !types
                .iter()
                .filter_map(Value::as_str)
                .any(|t| check_type(value, t, path).is_ok()) =>
        {
            return Err(format!("{path} has the wrong type"));
        }
        _ => {}
    }
    if let Some(allowed) = schema["enum"].as_array()
        && !allowed.contains(value)
    {
        return Err(format!(
            "{path} is {value}, which is not one of {}",
            schema["enum"]
        ));
    }
    if let Value::Object(map) = value {
        let properties = schema["properties"].as_object();
        for name in schema["required"].as_array().into_iter().flatten() {
            if let Some(name) = name.as_str()
                && !map.contains_key(name)
            {
                return Err(format!("{path}.{name} is missing"));
            }
        }
        for (name, v) in map {
            match properties.and_then(|p| p.get(name)) {
                Some(s) => check_schema(v, s, &format!("{path}.{name}"))?,
                None if schema["additionalProperties"] == Value::Bool(false) => {
                    return Err(format!("{path}.{name} is not allowed"));
                }
                None => {}
            }
        }
    }
    if let (Value::Array(items), Some(item_schema)) = (value, schema.get("items")) {
        for (i, v) in items.iter().enumerate() {
            check_schema(v, item_schema, &format!("{path}[{i}]"))?;
        }
    }
    Ok(())
}

fn check_type(value: &Value, expected: &str, path: &str) -> Result<(), String> {
    let ok = match expected {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        "number" => value.is_number(),
        "integer" => {
            value.is_i64() || value.is_u64() || value.as_f64().is_some_and(|f| f.fract() == 0.0)
        }
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{path} must be {expected}, but it is {value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_and_service_names_come_from_the_base_url() {
        let c = OpenAiConfig::new("http://localhost:11434/v1/", "llama");
        assert_eq!(c.chat_url(), "http://localhost:11434/v1/chat/completions");
        assert!(c.is_local());
        assert_eq!(c.service, "Local OpenAI-compatible server");
        assert_eq!(service_name(OPENAI_BASE_URL), "OpenAI API");
        assert_eq!(service_name("https://openrouter.ai/api/v1"), "OpenRouter");
        assert_eq!(
            service_name("https://generativelanguage.googleapis.com/v1beta/openai/"),
            "Google Gemini API"
        );
        assert_eq!(
            service_name("https://llm.example.com/v1"),
            "OpenAI-compatible API (llm.example.com)"
        );
        assert!(!OpenAiConfig::new(OPENAI_BASE_URL, "m").is_local());
    }

    #[test]
    fn structured_output_sets_the_default_retries() {
        let c = OpenAiConfig::new(OPENAI_BASE_URL, "m");
        assert_eq!(c.json_retries, 0);
        let c = c.with_structured_output(StructuredOutput::Prompt);
        assert_eq!(c.json_retries, 2);
    }

    #[test]
    fn json_is_found_inside_extra_text() {
        let v = json!({"ok": true});
        assert_eq!(parse_json(r#"{"ok": true}"#).unwrap(), v);
        assert_eq!(parse_json("```json\n{\"ok\": true}\n```").unwrap(), v);
        assert_eq!(
            parse_json("<think>{\"no\": 1}</think>\nHere it is: {\"ok\": true}").unwrap(),
            v
        );
        assert!(parse_json("no json here").is_err());
        assert!(parse_json("{ broken").is_err());
    }

    #[test]
    fn schema_check_covers_the_used_keywords() {
        let schema = json!({
            "type": "object",
            "properties": {
                "layers": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "index": { "type": "integer" },
                            "side": { "type": "string", "enum": ["left", "right"] },
                            "note": { "type": ["string", "null"] }
                        },
                        "required": ["index", "side"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["layers"],
            "additionalProperties": false
        });
        let ok = json!({"layers": [{"index": 1, "side": "left", "note": null}, {"index": 2.0, "side": "right"}]});
        assert_eq!(check_schema(&ok, &schema, "$"), Ok(()));
        for (bad, expected) in [
            (json!({}), "$.layers is missing"),
            (json!({"layers": [], "x": 1}), "$.x is not allowed"),
            (
                json!({"layers": [{"index": 1.5, "side": "left"}]}),
                "$.layers[0].index must be integer",
            ),
            (
                json!({"layers": [{"index": 1, "side": "up"}]}),
                "$.layers[0].side is \"up\"",
            ),
            (
                json!({"layers": [{"index": 1}]}),
                "$.layers[0].side is missing",
            ),
            (
                json!({"layers": [{"index": 1, "side": "left", "note": 3}]}),
                "$.layers[0].note has the wrong type",
            ),
            (json!([]), "$ must be object"),
        ] {
            let e = check_schema(&bad, &schema, "$").unwrap_err();
            assert!(e.starts_with(expected), "{e}");
        }
    }

    #[test]
    fn quota_errors_are_not_rate_limits() {
        let response = |code: &str| HttpResponse {
            status: 429,
            headers: vec![],
            body: json!({"error": {"message": "m", "type": "requests", "code": code}})
                .to_string()
                .into_bytes(),
        };
        assert!(is_quota_error(&response("insufficient_quota")));
        assert!(is_quota_error(&response("project_spend_limit_exceeded")));
        assert!(!is_quota_error(&response("rate_limit_exceeded")));
        let limited = HttpResponse {
            status: 429,
            headers: vec![],
            body: json!({"error": {"message": "slow down", "type": "rate_limit_error", "code": "slow_down"}})
                .to_string()
                .into_bytes(),
        };
        assert!(!is_quota_error(&limited));
    }

    #[test]
    fn error_details_from_various_servers() {
        let detail = |body: &str| {
            error_detail(&HttpResponse {
                status: 400,
                headers: vec![],
                body: body.as_bytes().to_vec(),
            })
        };
        assert_eq!(
            detail(r#"{"error":{"message":"bad","type":"invalid_request_error","code":null}}"#),
            ("invalid_request_error".into(), "bad".into())
        );
        assert_eq!(
            detail(r#"{"error":"model not found"}"#),
            ("unknown".into(), "model not found".into())
        );
        assert_eq!(
            detail(r#"{"object":"error","message":"oops","type":"BadRequestError"}"#),
            ("BadRequestError".into(), "oops".into())
        );
        assert_eq!(
            detail("<html>502</html>"),
            ("unknown".into(), "<html>502</html>".into())
        );
    }
}
