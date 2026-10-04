//! 提案の取り消しと時間の上限 (`docs/ai-integration.md#取り消しと時間の上限`)。録画を再生し、待つ関数は差し替えて
//! 実際には待たない。ネットワークは使わない。

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::json;
use ugoe_ai::ApiKey;
use ugoe_ai::claude::{ClaudeConfig, ClaudeProvider};
use ugoe_ai::depth::{DepthEstimator, DepthRequest, HttpDepthConfig, HttpDepthEstimator};
use ugoe_ai::openai::{OpenAiConfig, OpenAiProvider, StructuredOutput};
use ugoe_ai::transport::{
    Cassette, HttpRequest, HttpResponse, ReplayTransport, Transport, TransportError,
};
use ugoe_ai::{AiError, CancelToken, Provider, ProviderRequest};

fn cassette(name: &str) -> Cassette {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cassettes")
        .join(name);
    Cassette::load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn claude(transport: impl Transport + 'static) -> ClaudeProvider {
    ClaudeProvider::with_transport(
        ClaudeConfig::default(),
        ApiKey::new("sk-ant-test").unwrap(),
        transport,
    )
}

fn request() -> ProviderRequest {
    ProviderRequest {
        prompt: "hi".into(),
        ..Default::default()
    }
}

#[test]
fn a_cancelled_token_sends_nothing() {
    let transport = Arc::new(ReplayTransport::new(cassette("classify_success.json")));
    let cancel = CancelToken::new();
    cancel.cancel();
    let p = claude(transport.clone()).with_cancel(cancel);
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(matches!(e, AiError::Cancelled), "{e}");
    assert!(transport.received().is_empty());
}

#[test]
fn cancelling_during_the_retry_wait_stops_retrying() {
    // 429 (retry-after: 3) の後の待ちの間に取り消す
    let transport = Arc::new(ReplayTransport::new(cassette(
        "rate_limited_then_success.json",
    )));
    let cancel = CancelToken::new();
    let in_sleep = cancel.clone();
    let slept = Arc::new(Mutex::new(Vec::new()));
    let log = slept.clone();
    let p = claude(transport.clone())
        .with_cancel(cancel)
        .with_sleeper(move |d| {
            log.lock().unwrap().push(d);
            in_sleep.cancel();
        });
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(matches!(e, AiError::Cancelled), "{e}");
    assert_eq!(*slept.lock().unwrap(), [Duration::from_secs(3)]);
    assert_eq!(transport.received().len(), 1);
    assert_eq!(transport.remaining(), 2);
}

#[test]
fn a_zero_time_limit_sends_nothing() {
    let transport = Arc::new(ReplayTransport::new(cassette("classify_success.json")));
    let p = claude(transport.clone()).with_cancel(CancelToken::with_time_limit(Duration::ZERO));
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(
        matches!(e, AiError::TimeLimitExceeded { limit } if limit == Duration::ZERO),
        "{e}"
    );
    assert!(transport.received().is_empty());
}

#[test]
fn does_not_wait_past_the_time_limit() {
    // retry-after (3 秒) が残り (1 秒) を超えるので、待たずに上限のエラーにする
    let transport = Arc::new(ReplayTransport::new(cassette(
        "rate_limited_then_success.json",
    )));
    let limit = Duration::from_secs(1);
    let p = claude(transport.clone())
        .with_cancel(CancelToken::with_time_limit(limit))
        .with_sleeper(|_| panic!("must not wait"));
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    assert!(
        matches!(e, AiError::TimeLimitExceeded { limit: l } if l == limit),
        "{e}"
    );
    assert_eq!(transport.remaining(), 2);
}

/// 受け取ったリクエストの時間の上限を覚え、`inner` に渡す。
struct Capture<T> {
    inner: T,
    timeouts: Mutex<Vec<Option<Duration>>>,
}

impl<T: Transport> Transport for Capture<T> {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        self.timeouts.lock().unwrap().push(request.timeout);
        self.inner.send(request)
    }
}

#[test]
fn each_request_is_limited_to_the_remaining_time() {
    let capture = |name| {
        Arc::new(Capture {
            inner: ReplayTransport::new(cassette(name)),
            timeouts: Mutex::default(),
        })
    };
    let limited = capture("classify_success.json");
    let limit = Duration::from_secs(600);
    claude(limited.clone())
        .with_cancel(CancelToken::with_time_limit(limit))
        .complete_json(&request(), &json!({}))
        .ok();
    let timeouts = limited.timeouts.lock().unwrap().clone();
    assert_eq!(timeouts.len(), 1);
    assert!(timeouts[0].is_some_and(|t| t <= limit && t > Duration::ZERO));

    let unlimited = capture("classify_success.json");
    claude(unlimited.clone())
        .complete_json(&request(), &json!({}))
        .ok();
    assert_eq!(*unlimited.timeouts.lock().unwrap(), [None]);
}

/// 応答を返さずに止まる通信 (送り始めたことを `entered` で知らせる)。
struct Stuck {
    entered: Mutex<mpsc::Sender<()>>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl Transport for Stuck {
    fn send(&self, _request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        self.entered.lock().unwrap().send(()).unwrap();
        // テストの終わりに送り手が消えるまで止まる
        let _ = self.release.lock().unwrap().recv();
        Err(TransportError::new(
            ugoe_ai::transport::TransportErrorKind::Timeout,
            "released",
        ))
    }
}

#[test]
fn cancelling_stops_waiting_for_a_response_in_flight() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let transport = Stuck {
        entered: Mutex::new(entered_tx),
        release: Mutex::new(release_rx),
    };
    let cancel = CancelToken::new();
    let p = claude(transport).with_cancel(cancel.clone());
    let canceller = std::thread::spawn(move || {
        entered_rx.recv().unwrap();
        cancel.cancel();
    });
    let started = Instant::now();
    let e = p.complete_json(&request(), &json!({})).unwrap_err();
    canceller.join().unwrap();
    assert!(matches!(e, AiError::Cancelled), "{e}");
    // 通信の終わり (ここでは来ない) を待たずに返る
    assert!(started.elapsed() < Duration::from_secs(30));
    drop(release_tx);
}

/// 1 回送ったら印を取り消す通信。
struct CancelAfterFirst<T> {
    inner: T,
    cancel: CancelToken,
}

impl<T: Transport> Transport for CancelAfterFirst<T> {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        let response = self.inner.send(request);
        self.cancel.cancel();
        response
    }
}

#[test]
fn cancelling_stops_asking_again() {
    // プロンプトにスキーマを書くサーバ: 1 回目の答えが検証を通らず問い直すところで取り消されている
    let replay = Arc::new(ReplayTransport::new(cassette(
        "openai_ollama_prompt_retry.json",
    )));
    let cancel = CancelToken::new();
    let config = OpenAiConfig::new("http://localhost:11434/v1", "qwen3-vl:8b")
        .with_structured_output(StructuredOutput::Prompt);
    let p = OpenAiProvider::with_transport(
        config,
        None,
        CancelAfterFirst {
            inner: replay.clone(),
            cancel: cancel.clone(),
        },
    )
    .with_cancel(cancel);
    let request = ProviderRequest {
        system: "You label the layers of a character illustration.".into(),
        prompt: "Label the layer named \"eye_l\".".into(),
        images: vec![b"\x89PNG\r\n\x1a\nIHDR".to_vec()],
    };
    let schema = json!({
        "type": "object",
        "properties": {
            "label": { "type": "string" },
            "side": { "type": "string", "enum": ["left", "right", "center"] }
        },
        "required": ["label", "side"],
        "additionalProperties": false
    });
    let e = p.complete_json(&request, &schema).unwrap_err();
    assert!(matches!(e, AiError::Cancelled), "{e}");
    assert_eq!(replay.received().len(), 1);
    assert_eq!(replay.remaining(), 1);
}

#[test]
fn depth_estimation_can_be_cancelled() {
    let transport = Arc::new(ReplayTransport::new(cassette("depth_face.json")));
    let cancel = CancelToken::new();
    cancel.cancel();
    let estimator = HttpDepthEstimator::with_transport(
        HttpDepthConfig::new("http://127.0.0.1:8765/depth", "depth-anything-v2-small"),
        transport.clone(),
    )
    .with_cancel(cancel);
    let e = estimator
        .estimate(&DepthRequest {
            image: b"\x89PNG\r\n\x1a\nIHDR".to_vec(),
        })
        .unwrap_err();
    assert!(matches!(e, AiError::Cancelled), "{e}");
    assert!(transport.received().is_empty());
}
