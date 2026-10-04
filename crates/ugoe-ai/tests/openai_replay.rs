//! 録画した OpenAI 互換の API の応答を再生して、[`OpenAiProvider`] を確かめる。ネットワークは使わない。
//!
//! 録画は `tests/cassettes/openai_*.json` にある (Chat Completions の応答の形に合わせて手で作ったもの。
//! `openai_ollama_*` は Ollama の OpenAI 互換の窓口の形)。実際の API から録り直すときは `record_live` を参照。

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use ugoe_ai::ApiKey;
use ugoe_ai::openai::{
    MaxTokensField, OPENAI_BASE_URL, OpenAiConfig, OpenAiProvider, StructuredOutput,
};
use ugoe_ai::transport::{Cassette, RecordingTransport, ReplayTransport};
use ugoe_ai::{AiError, Destination, Provider, ProviderRequest};

const SECRET: &str = "sk-test-DO-NOT-LEAK";
const MODEL: &str = "gpt-6-sol";
const OLLAMA: &str = "http://localhost:11434/v1";

fn cassette(name: &str) -> Cassette {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cassettes")
        .join(name);
    Cassette::load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn config() -> OpenAiConfig {
    OpenAiConfig::new(OPENAI_BASE_URL, MODEL)
}

type Slept = Arc<Mutex<Vec<Duration>>>;

/// 再生用のプロバイダと、受け取ったリクエスト・待った時間の記録。
fn provider(
    config: OpenAiConfig,
    key: Option<&str>,
    name: &str,
) -> (OpenAiProvider, Arc<ReplayTransport>, Slept) {
    let transport = Arc::new(ReplayTransport::new(cassette(name)));
    let slept = Slept::default();
    let log = slept.clone();
    let p = OpenAiProvider::with_transport(
        config,
        key.map(|k| ApiKey::new(k).unwrap()),
        transport.clone(),
    )
    .with_sleeper(move |d| log.lock().unwrap().push(d));
    (p, transport, slept)
}

fn schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "label": { "type": "string" },
            "side": { "type": "string", "enum": ["left", "right", "center"] }
        },
        "required": ["label", "side"],
        "additionalProperties": false
    })
}

/// PNG の署名だけを持つ小さなバイト列 (中身は検査しない)。
fn png() -> Vec<u8> {
    b"\x89PNG\r\n\x1a\nIHDR".to_vec()
}

fn request() -> ProviderRequest {
    ProviderRequest {
        system: "You label the layers of a character illustration.".into(),
        prompt: "Label the layer named \"eye_l\".".into(),
        images: vec![png()],
    }
}

fn assert_no_secret(e: &AiError) {
    assert!(!e.to_string().contains(SECRET), "{e}");
    assert!(!format!("{e:?}").contains(SECRET), "{e:?}");
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

#[test]
fn sends_image_as_data_url_and_parses_structured_output() {
    // 録画にはリクエストの本文も書いてあり (画像は data URL の長さだけ)、送ったものと一致しないと失敗する
    let (p, transport, slept) = provider(config(), Some(SECRET), "openai_success.json");
    let out = p.complete_json(&request(), &schema()).unwrap();
    assert_eq!(out, json!({ "label": "iris", "side": "left" }));
    assert!(slept.lock().unwrap().is_empty());

    let received = transport.received();
    let headers = &received[0].headers;
    assert_eq!(header(headers, "authorization"), Some("<redacted>"));
    assert_eq!(header(headers, "content-type"), Some("application/json"));
    assert!(!format!("{received:?}").contains(SECRET));
    assert!(!format!("{p:?}").contains(SECRET));
}

#[test]
fn local_servers_need_no_key() {
    let config =
        OpenAiConfig::new(OLLAMA, "qwen3-vl:8b").with_structured_output(StructuredOutput::Prompt);
    let (p, transport, _) = provider(config, None, "openai_ollama_prompt_retry.json");
    p.complete_json(&request(), &schema()).unwrap();
    let headers = &transport.received()[0].headers;
    assert_eq!(header(headers, "authorization"), None);
}

/// 構造化出力に対応しないサーバ: スキーマはプロンプトに書き、答えを検証して、通らなければ前の答えと
/// 理由を添えて問い直す。考えた内容やコードフェンスは除いて読む。
#[test]
fn prompt_mode_validates_and_asks_again() {
    let config =
        OpenAiConfig::new(OLLAMA, "qwen3-vl:8b").with_structured_output(StructuredOutput::Prompt);
    let (p, transport, _) = provider(config, None, "openai_ollama_prompt_retry.json");
    let out = p.complete_json(&request(), &schema()).unwrap();
    assert_eq!(out, json!({ "label": "iris", "side": "left" }));
    assert_eq!(transport.remaining(), 0);

    let received = transport.received();
    let first = received[0].body.as_ref().unwrap();
    assert!(first.get("response_format").is_none());
    let prompt = first["messages"][1]["content"][1]["text"].as_str().unwrap();
    assert!(prompt.contains("matches this JSON Schema"), "{prompt}");
    assert!(
        prompt.contains(r#""enum":["left","right","center"]"#),
        "{prompt}"
    );

    // 2 回目は、同じ画像と前の答え・理由を添えて送る
    let second = received[1].body.as_ref().unwrap();
    let messages = second["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[2]["role"], "assistant");
    assert!(messages[2]["content"].as_str().unwrap().contains("LEFT"));
    let reason = messages[3]["content"].as_str().unwrap();
    assert!(reason.contains("$.side is \"LEFT\""), "{reason}");
    assert_eq!(messages[1], first["messages"][1]);
}

#[test]
fn prompt_mode_gives_up_after_the_retries() {
    let mut config =
        OpenAiConfig::new(OLLAMA, "qwen3-vl:8b").with_structured_output(StructuredOutput::Prompt);
    config.json_retries = 0;
    let (p, transport, _) = provider(config, None, "openai_ollama_prompt_retry.json");
    let e = p.complete_json(&request(), &schema()).unwrap_err();
    assert!(
        matches!(&e, AiError::InvalidResponse(m) if m.contains("does not match the schema")),
        "{e:?}"
    );
    assert_eq!(transport.remaining(), 1);
}

#[test]
fn json_object_mode_and_token_limit_fields() {
    let mut config = config().with_structured_output(StructuredOutput::JsonObject);
    config.max_tokens = Some(2048);
    let (p, transport, _) = provider(
        config.clone(),
        Some(SECRET),
        "openai_rate_limited_then_success.json",
    );
    p.complete_json(
        &ProviderRequest {
            prompt: "hi".into(),
            ..Default::default()
        },
        &json!({}),
    )
    .unwrap();
    let body = transport.received()[0].body.clone().unwrap();
    assert_eq!(body["response_format"], json!({ "type": "json_object" }));
    assert_eq!(body["max_completion_tokens"], 2048);
    assert_eq!(body["model"], MODEL);
    // システムプロンプトが空なら送らない。JSON モードは「JSON」という語を要る
    assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    assert!(
        body["messages"][0]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("JSON")
    );

    config.max_tokens_field = MaxTokensField::MaxTokens;
    let (p, transport, _) = provider(
        config,
        Some(SECRET),
        "openai_rate_limited_then_success.json",
    );
    p.complete_json(&ProviderRequest::default(), &json!({}))
        .unwrap();
    let body = transport.received()[0].body.clone().unwrap();
    assert_eq!(body["max_tokens"], 2048);
    assert!(body.get("max_completion_tokens").is_none());
}

#[test]
fn retries_rate_limits_and_overload() {
    let (p, _, slept) = provider(
        config(),
        Some(SECRET),
        "openai_rate_limited_then_success.json",
    );
    let out = p.complete_json(&request(), &json!({})).unwrap();
    assert_eq!(out, json!({ "ok": true }));
    // 429 の後は retry-after (3 秒)、503 の後は 2 回目のバックオフ (1 秒) 待つ
    assert_eq!(
        *slept.lock().unwrap(),
        vec![Duration::from_secs(3), Duration::from_secs(1)]
    );
}

#[test]
fn gives_up_on_rate_limit_when_retry_after_is_too_long() {
    let mut config = config();
    config.retry.max_retry_after = Duration::from_secs(1);
    let (p, _, slept) = provider(
        config,
        Some(SECRET),
        "openai_rate_limited_then_success.json",
    );
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(
        matches!(e, AiError::RateLimited { retry_after: Some(d) } if d == Duration::from_secs(3)),
        "{e:?}"
    );
    assert!(slept.lock().unwrap().is_empty());
}

/// 残高の不足は 429 でも再試行せず、理由を示す。
#[test]
fn does_not_retry_quota_errors() {
    let (p, transport, slept) = provider(config(), Some(SECRET), "openai_quota.json");
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(
        matches!(&e, AiError::Api { status: 429, kind, .. } if kind == "insufficient_quota"),
        "{e:?}"
    );
    assert!(e.to_string().contains("exceeded your current quota"));
    assert!(slept.lock().unwrap().is_empty());
    assert_eq!(transport.received().len(), 1);
}

#[test]
fn does_not_retry_auth_errors_or_leak_the_key() {
    let (p, _, slept) = provider(config(), Some(SECRET), "openai_unauthorized.json");
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(
        matches!(&e, AiError::Api { status: 401, kind, .. } if kind == "invalid_api_key"),
        "{e:?}"
    );
    assert!(e.to_string().contains("<redacted>"));
    assert_no_secret(&e);
    assert!(slept.lock().unwrap().is_empty());
}

#[test]
fn reports_refusals_and_truncation() {
    let (p, _, _) = provider(config(), Some(SECRET), "openai_refusal.json");
    let e = p.complete_json(&request(), &schema()).unwrap_err();
    assert!(matches!(e, AiError::Refused { category: None }), "{e:?}");

    let (p, _, _) = provider(config(), Some(SECRET), "openai_length.json");
    let e = p.complete_json(&request(), &schema()).unwrap_err();
    assert!(matches!(e, AiError::Truncated), "{e:?}");
}

#[test]
fn rejects_non_png_images_before_sending() {
    let (p, transport, _) = provider(config(), Some(SECRET), "openai_success.json");
    let req = ProviderRequest {
        images: vec![b"GIF89a".to_vec()],
        ..request()
    };
    let e = p.complete_json(&req, &schema()).unwrap_err();
    assert!(matches!(e, AiError::InvalidRequest(_)), "{e:?}");
    assert!(transport.received().is_empty());
}

#[test]
fn network_errors_do_not_leak_the_key() {
    let p = OpenAiProvider::with_transport(
        config(),
        Some(ApiKey::new(SECRET).unwrap()),
        ReplayTransport::new(Cassette::default()),
    );
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(matches!(e, AiError::Network(_)), "{e:?}");
    assert_no_secret(&e);
}

/// 送り先がループバックなら外部に送らない扱い。それ以外は送り先とモデルを示す。
#[test]
fn discloses_external_destinations_only() {
    let (p, _, _) = provider(config(), Some(SECRET), "openai_success.json");
    let d = p.disclosure(&request());
    assert_eq!(
        d.destination,
        Destination::External {
            service: "OpenAI API".into(),
            endpoint: "https://api.openai.com/v1/chat/completions".into(),
            model: MODEL.into(),
        }
    );
    assert_eq!((d.image_count, d.image_bytes), (1, png().len()));

    for url in [OLLAMA, "http://127.0.0.1:1234/v1", "http://[::1]:8000/v1"] {
        let p = OpenAiProvider::new(OpenAiConfig::new(url, "m"), None);
        assert!(!p.disclosure(&request()).is_external(), "{url}");
    }
    let lan = OpenAiProvider::new(OpenAiConfig::new("http://192.168.1.20:11434/v1", "m"), None);
    assert!(lan.disclosure(&request()).is_external());
}

#[test]
fn recording_keeps_neither_the_key_nor_the_images() {
    let rec = Arc::new(RecordingTransport::new(ReplayTransport::new(cassette(
        "openai_success.json",
    ))));
    let p =
        OpenAiProvider::with_transport(config(), Some(ApiKey::new(SECRET).unwrap()), rec.clone());
    p.complete_json(&request(), &schema()).unwrap();
    let recorded = rec.cassette();
    // 録画をそのまま再生できる (リクエストの本文も一致する)
    assert_eq!(recorded, cassette("openai_success.json"));
    let json = recorded.to_json().unwrap();
    assert!(!json.contains(SECRET));
    assert!(
        !json.contains("iVBORw0KGgo"),
        "the PNG data must not be recorded"
    );
}

/// 実際の API から録画を作る。既定では動かさない。
///
/// ```sh
/// OPENAI_API_KEY=... UGOE_AI_RECORD_TO=/tmp/live.json \
///   UGOE_AI_OPENAI_BASE_URL=https://api.openai.com/v1 UGOE_AI_OPENAI_MODEL=gpt-6-sol \
///   cargo test -p ugoe-ai --test openai_replay -- --ignored record_live
/// ```
///
/// 手元の Ollama なら `UGOE_AI_OPENAI_BASE_URL=http://localhost:11434/v1` とし、キーは要らない。
#[test]
#[ignore = "calls a real OpenAI-compatible API"]
fn record_live() {
    let out = std::env::var("UGOE_AI_RECORD_TO").expect("set UGOE_AI_RECORD_TO");
    let base = std::env::var("UGOE_AI_OPENAI_BASE_URL").unwrap_or_else(|_| OPENAI_BASE_URL.into());
    let model = std::env::var("UGOE_AI_OPENAI_MODEL").unwrap_or_else(|_| MODEL.into());
    let config = OpenAiConfig::new(base, model);
    let transport = ugoe_ai::transport::UreqTransport::new(config.timeout);
    let rec = Arc::new(RecordingTransport::new(transport));
    let key = ApiKey::from_env(ugoe_ai::openai::API_KEY_ENV).ok();
    let p = OpenAiProvider::with_transport(config, key, rec.clone());
    let req = ProviderRequest {
        images: vec![],
        ..request()
    };
    let result = p.complete_json(&req, &schema());
    rec.cassette().save(&out).unwrap();
    println!("{result:?}");
}
