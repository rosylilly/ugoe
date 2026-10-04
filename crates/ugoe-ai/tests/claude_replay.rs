//! 録画した Claude API の応答を再生して、[`ClaudeProvider`] を確かめる。ネットワークは使わない。
//!
//! 録画は `tests/cassettes/` にある (Messages API の応答の形に合わせて手で作ったもの)。
//! 実際の API から録り直すときは `record_live` を参照。

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use ugoe_ai::ApiKey;
use ugoe_ai::claude::{self, ClaudeConfig, ClaudeProvider, Effort};
use ugoe_ai::transport::{Cassette, RecordingTransport, ReplayTransport};
use ugoe_ai::{AiError, Destination, Provider, ProviderRequest};

const SECRET: &str = "sk-ant-test-DO-NOT-LEAK";

fn cassette(name: &str) -> Cassette {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cassettes")
        .join(name);
    Cassette::load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// 再生用のプロバイダと、待った時間の記録。
fn provider(config: ClaudeConfig, name: &str) -> (ClaudeProvider, Arc<Mutex<Vec<Duration>>>) {
    let slept = Arc::new(Mutex::new(Vec::new()));
    let log = slept.clone();
    let p = ClaudeProvider::with_transport(
        config,
        ApiKey::new(SECRET).unwrap(),
        ReplayTransport::new(cassette(name)),
    )
    .with_sleeper(move |d| log.lock().unwrap().push(d));
    (p, slept)
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

/// エラーの表示と Debug の両方に API キーが含まれないこと。
fn assert_no_secret(e: &AiError) {
    assert!(!e.to_string().contains(SECRET), "{e}");
    assert!(!format!("{e:?}").contains(SECRET), "{e:?}");
}

#[test]
fn sends_image_and_schema_and_parses_structured_output() {
    // 録画にはリクエストの本文も書いてあり、送ったものと一致しないと再生が失敗する
    let (p, slept) = provider(ClaudeConfig::default(), "classify_success.json");
    let out = p.complete_json(&request(), &schema()).unwrap();
    assert_eq!(out, json!({ "label": "iris", "side": "left" }));
    assert!(slept.lock().unwrap().is_empty());
}

#[test]
fn sends_required_headers_without_exposing_the_key() {
    let transport = Arc::new(ReplayTransport::new(cassette("classify_success.json")));
    let p = ClaudeProvider::with_transport(
        ClaudeConfig::default(),
        ApiKey::new(SECRET).unwrap(),
        transport.clone(),
    );
    p.complete_json(&request(), &schema()).unwrap();
    let received = transport.received();
    assert_eq!(received.len(), 1);
    let headers = &received[0].headers;
    let get = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(get("anthropic-version"), Some("2023-06-01"));
    assert_eq!(get("content-type"), Some("application/json"));
    assert_eq!(get("x-api-key"), Some("<redacted>"));
    assert!(!format!("{received:?}").contains(SECRET));
    assert!(!format!("{p:?}").contains(SECRET));
}

#[test]
fn model_and_effort_come_from_the_config() {
    let config = ClaudeConfig {
        model: "claude-sonnet-5".into(),
        effort: Some(Effort::Low),
        max_tokens: 2048,
        ..Default::default()
    };
    let transport = Arc::new(ReplayTransport::new(cassette(
        "rate_limited_then_success.json",
    )));
    let p = ClaudeProvider::with_transport(config, ApiKey::new(SECRET).unwrap(), transport.clone())
        .with_sleeper(|_| {});
    p.complete_json(
        &ProviderRequest {
            prompt: "hi".into(),
            ..Default::default()
        },
        &json!({}),
    )
    .unwrap();
    let body = transport.received()[0].body.clone().unwrap();
    assert_eq!(body["model"], "claude-sonnet-5");
    assert_eq!(body["max_tokens"], 2048);
    assert_eq!(body["output_config"]["effort"], "low");
    // システムプロンプトが空なら送らない
    assert!(body.get("system").is_none());
}

#[test]
fn retries_rate_limits_and_timeouts() {
    let (p, slept) = provider(ClaudeConfig::default(), "rate_limited_then_success.json");
    let out = p.complete_json(&request(), &json!({})).unwrap();
    assert_eq!(out, json!({ "ok": true }));
    // 429 の後は retry-after (3 秒)、タイムアウトの後は 2 回目のバックオフ (1 秒) 待つ
    assert_eq!(
        *slept.lock().unwrap(),
        vec![Duration::from_secs(3), Duration::from_secs(1)]
    );
}

#[test]
fn gives_up_on_rate_limit_when_retry_after_is_too_long() {
    let mut config = ClaudeConfig::default();
    config.retry.max_retry_after = Duration::from_secs(1);
    let (p, slept) = provider(config, "rate_limited_then_success.json");
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(
        matches!(e, AiError::RateLimited { retry_after: Some(d) } if d == Duration::from_secs(3)),
        "{e:?}"
    );
    assert!(slept.lock().unwrap().is_empty());
}

#[test]
fn stops_after_max_retries() {
    let (p, slept) = provider(ClaudeConfig::default(), "overloaded.json");
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(
        matches!(&e, AiError::Api { status: 529, kind, .. } if kind == "overloaded_error"),
        "{e:?}"
    );
    // 最初の試行 + 2 回の再試行
    assert_eq!(
        *slept.lock().unwrap(),
        vec![Duration::from_millis(500), Duration::from_secs(1)]
    );
}

#[test]
fn does_not_retry_auth_errors_or_leak_the_key() {
    let (p, slept) = provider(ClaudeConfig::default(), "unauthorized.json");
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(matches!(&e, AiError::Api { status: 401, .. }), "{e:?}");
    assert!(e.to_string().contains("<redacted>"));
    assert_no_secret(&e);
    assert!(slept.lock().unwrap().is_empty());
}

#[test]
fn reports_refusals() {
    let (p, _) = provider(ClaudeConfig::default(), "refusal.json");
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(
        matches!(&e, AiError::Refused { category: Some(c) } if c == "cyber"),
        "{e:?}"
    );
}

#[test]
fn reports_truncated_output() {
    let (p, _) = provider(ClaudeConfig::default(), "max_tokens.json");
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(matches!(e, AiError::Truncated), "{e:?}");
}

#[test]
fn rejects_non_png_images_before_sending() {
    let transport = Arc::new(ReplayTransport::new(cassette("classify_success.json")));
    let p = ClaudeProvider::with_transport(
        ClaudeConfig::default(),
        ApiKey::new(SECRET).unwrap(),
        transport.clone(),
    );
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
    // 録画が尽きると再試行しない失敗になる
    let p = ClaudeProvider::with_transport(
        ClaudeConfig::default(),
        ApiKey::new(SECRET).unwrap(),
        ReplayTransport::new(Cassette::default()),
    );
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(matches!(e, AiError::Network(_)), "{e:?}");
    assert_no_secret(&e);
}

#[test]
fn discloses_what_is_sent() {
    let (p, _) = provider(ClaudeConfig::default(), "classify_success.json");
    let d = p.disclosure(&request());
    assert!(d.is_external());
    assert_eq!(
        d.destination,
        Destination::External {
            service: "Anthropic Claude API".into(),
            endpoint: "https://api.anthropic.com/v1/messages".into(),
            model: "claude-opus-5-5".into(),
        }
    );
    assert_eq!(d.image_count, 1);
    assert_eq!(d.image_bytes, png().len());
}

#[test]
fn recording_keeps_neither_the_key_nor_the_images() {
    let rec = Arc::new(RecordingTransport::new(ReplayTransport::new(cassette(
        "classify_success.json",
    ))));
    let p = ClaudeProvider::with_transport(
        ClaudeConfig::default(),
        ApiKey::new(SECRET).unwrap(),
        rec.clone(),
    );
    p.complete_json(&request(), &schema()).unwrap();
    let recorded = rec.cassette();
    // 録画をそのまま再生できる (リクエストの本文も一致する)
    assert_eq!(recorded, cassette("classify_success.json"));
    let json = recorded.to_json().unwrap();
    assert!(!json.contains(SECRET));
    assert!(
        !json.contains("iVBORw0KGgo"),
        "the PNG data must not be recorded"
    );
}

/// 実際の API から録画を作る。キーが要るので既定では動かさない。
///
/// ```sh
/// ANTHROPIC_API_KEY=... UGOE_AI_RECORD_TO=/tmp/live.json \
///   cargo test -p ugoe-ai --test claude_replay -- --ignored record_live
/// ```
#[test]
#[ignore = "calls the real Claude API"]
fn record_live() {
    let out = std::env::var("UGOE_AI_RECORD_TO").expect("set UGOE_AI_RECORD_TO");
    let config = ClaudeConfig::default();
    let transport = ugoe_ai::transport::UreqTransport::new(config.timeout);
    let rec = Arc::new(RecordingTransport::new(transport));
    let p = ClaudeProvider::with_transport(
        config,
        ApiKey::from_env(claude::API_KEY_ENV).unwrap(),
        rec.clone(),
    );
    let req = ProviderRequest {
        images: vec![],
        ..request()
    };
    let result = p.complete_json(&req, &schema());
    rec.cassette().save(&out).unwrap();
    println!("{result:?}");
}
