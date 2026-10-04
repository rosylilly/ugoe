//! 深度推定による振り向きの提案を、録画した深度マップで確かめる。ネットワークは使わない。
//!
//! 録画 `tests/cassettes/depth_face.json` は深度推定サーバの応答の形に合わせて作ったもの
//! (`tests/common/mod.rs` の顔に合わせ、顔を楕円体、後ろ髪を顔より奥の板、前髪を額と同じ深さにした
//! 32×32 の相対深度。値が大きいほど手前)。前髪を額と同じ深さにしたのは、平たく塗った絵で単眼の
//! 推定が前後を取り違える様子をまねるため (ラベルによる補正で前髪が手前に出ることを確かめる)。

#![allow(clippy::unwrap_used)]

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use ugoe_ai::classify::{Label, LayerLabel, Side};

use ugoe_ai::ApiKey;
use ugoe_ai::depth::{
    DepthEstimator, DepthMap, DepthRequest, FixedDepthEstimator, HttpDepthConfig,
    HttpDepthEstimator, RetryPolicy,
};
use ugoe_ai::transport::{Cassette, RecordingTransport, ReplayTransport};
use ugoe_ai::turn::{TurnRole, TurnTarget, mesh_labels, plan_turn, propose_depth_turn};
use ugoe_ai::{AiError, Destination, Proposal};
use ugoe_core::Vec2;
use ugoe_editor_core::{Command, Document};
use ugoe_model::{DeformerKind, KeyformInference, KeyformOrigin, Model};
use ugoe_runtime::{CompiledModel, ModelInstance};

use common::{face_labels, face_model, png};

const ENDPOINT: &str = "http://127.0.0.1:8765/depth";

fn estimator() -> HttpDepthEstimator {
    HttpDepthEstimator::with_transport(
        HttpDepthConfig::new(ENDPOINT, "depth-anything-v2-small"),
        ReplayTransport::new(common::cassette("depth_face.json")),
    )
}

fn face() -> TurnTarget {
    TurnTarget {
        deformer: "Face".into(),
        role: None,
    }
}

fn propose(model: &Model) -> Proposal {
    propose_depth_turn(
        model,
        &estimator(),
        &DepthRequest { image: png() },
        &face_labels(),
        &[face()],
    )
    .unwrap()
}

fn apply(model: Model, proposal: &Proposal) -> Model {
    let mut doc = Document::new(model);
    doc.apply(Command::Batch {
        label: proposal.title.clone(),
        commands: proposal.commands.clone(),
    })
    .unwrap();
    doc.model().clone()
}

fn depth_of(proposal: &Proposal) -> Vec<f32> {
    proposal
        .commands
        .iter()
        .find_map(|c| match c {
            Command::SetDepth { depth, .. } => depth.clone(),
            _ => None,
        })
        .unwrap()
}

/// パラメータを `values` にしたときの、ワープ `Face` の制御点 (キャンバス座標)。
fn face_points(model: &Model, values: &[(&str, f32)]) -> Vec<Vec2> {
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    for (id, v) in values {
        let index = instance.parameters().index_of(id).unwrap();
        instance.parameters_mut().set(index, *v);
    }
    instance.update();
    let face = instance.compiled().deformer_index("Face").unwrap();
    instance.warp_world_points(face).unwrap().to_vec()
}

/// 5×5 の制御点の添字。
fn at(x: usize, y: usize) -> usize {
    y * 5 + x
}

#[test]
fn the_proposal_validates_and_turns_the_face() {
    let model = face_model();
    let proposal = propose(&model);
    let applied = apply(model, &proposal);
    assert!(ugoe_model::validate::validate(&applied).is_empty());

    // 格子は AngleX × AngleY の 9 セルで、基準のセルのほかは深度からの回転で作ったもの。
    let DeformerKind::Warp(warp) = &applied.deformers[0].kind else {
        panic!("Face is not a warp");
    };
    let grid = &warp.keyforms;
    let axes: Vec<_> = grid.bindings.iter().map(|b| b.parameter.as_str()).collect();
    assert_eq!(axes, ["AngleX", "AngleY"]);
    assert_eq!(grid.keyforms.len(), 9);
    for i in 0..9 {
        let expected = if i == 4 {
            KeyformOrigin::Authored
        } else {
            KeyformOrigin::DepthRotation
        };
        assert_eq!(grid.origin(i), expected, "cell {i}");
    }
    let Some(KeyformInference::DepthRotation(rotation)) = &grid.inference else {
        panic!("the grid does not use the depth rotation");
    };
    // 回転の中心は顔の楕円の中心 (扇の頂点の外接矩形の中心)。
    assert!((rotation.pivot - common::FACE_CENTER).length() < 1.0);
    assert_eq!(rotation.angle_scale, 1.0);

    let depth = warp.depth.clone().unwrap();
    let base = face_points(&applied, &[]);
    let pivot = rotation.pivot;
    let (sin, cos) = 30f32.to_radians().sin_cos();

    // 顔の中央の制御点 (128, 128) は最も手前にあり、AngleX = +30 で右へ d·sin30 だけ動く。
    let center = at(2, 2);
    let d = depth[center];
    assert!(d > 30.0, "center depth {d}");
    let right = face_points(&applied, &[("AngleX", 30.0)]);
    let left = face_points(&applied, &[("AngleX", -30.0)]);
    let expected = pivot.x + (base[center].x - pivot.x) * cos + d * sin;
    assert!((right[center].x - expected).abs() < 1e-2);
    assert!(right[center].x - base[center].x > 15.0);
    assert!((left[center].x - (2.0 * pivot.x - expected)).abs() < 1e-2);
    assert!((right[center].y - base[center].y).abs() < 1e-3);

    // AngleY = +30 で上 (-y) を向く。
    let up = face_points(&applied, &[("AngleY", 30.0)]);
    let expected = pivot.y + (base[center].y - pivot.y) * cos - d * sin;
    assert!((up[center].y - expected).abs() < 1e-2);
    assert!(up[center].y < base[center].y - 15.0);

    // 後ろ髪だけに覆われた制御点 (40, 128) は輪郭より奥なので、右を向くと左へ回り込む
    // (平らな板を回したときより左に出る)。
    let hair = at(0, 2);
    assert!(depth[hair] < 0.0);
    let flat = pivot.x + (base[hair].x - pivot.x) * cos;
    assert!(right[hair].x < flat - 1.0);

    // 何にも覆われていない角は深度 0 (輪郭と同じ) で、中心に向かって cos 倍縮むだけ。
    let corner = at(0, 0);
    assert_eq!(depth[corner], 0.0);
    let flat = pivot.x + (base[corner].x - pivot.x) * cos;
    assert!((right[corner].x - flat).abs() < 1e-2);

    // 斜めのセル (30, 30) では中央の点が右上へ動く。
    let diagonal = face_points(&applied, &[("AngleX", 30.0), ("AngleY", 30.0)]);
    assert!(diagonal[center].x > base[center].x + 10.0);
    assert!(diagonal[center].y < base[center].y - 10.0);
}

#[test]
fn labels_correct_the_estimated_depth() {
    let model = face_model();
    let map = estimator()
        .estimate(&DepthRequest { image: png() })
        .unwrap();
    let plan = plan_turn(&model, &map, &face_labels(), &face()).unwrap();
    assert_eq!(plan.role, TurnRole::Head);
    // 起伏は顔の幅 (144px) の 0.3 倍。
    assert!((plan.relief - 0.3 * 144.0).abs() < 1.0, "{}", plan.relief);

    // 前髪の制御点 (128, 76) は、推定では額と同じ深さでも、顔の起伏の 0.8 倍より手前に出る。
    let bangs = at(2, 1);
    assert_eq!(plan.labels[bangs], Some(Label::HairFront));
    assert!(plan.depth[bangs] >= 0.8 * plan.relief - 1e-3);
    // 前髪のラベルがなければ推定どおり (前髪の下の額と同じ) になる。
    let mut labels = face_labels();
    labels.remove(&"Bangs".into());
    labels.remove(&"HairBack".into());
    let unlabeled = plan_turn(&model, &map, &labels, &face()).unwrap();
    assert_eq!(unlabeled.labels[bangs], None);
    assert!(unlabeled.depth[bangs] < plan.depth[bangs] - 1.0);

    // 後ろ髪は輪郭 (0) より奥。
    let hair = at(0, 2);
    assert_eq!(plan.labels[hair], Some(Label::HairBack));
    assert!((plan.depth[hair] + 0.3 * plan.relief).abs() < 1e-3);
    assert!(unlabeled.depth[hair] >= 0.0);

    // 顔の制御点は推定の順序を保つ (中央が最も手前、輪郭に近いほど奥)。
    assert_eq!(plan.labels[at(2, 2)], Some(Label::Face));
    assert!(plan.depth[at(2, 2)] > plan.depth[at(3, 2)]);
    assert!(plan.depth[at(3, 2)] > 0.0);
}

#[test]
fn missing_parameters_are_added() {
    let mut model = face_model();
    model.parameters.clear();
    let proposal = propose(&model);
    let added: Vec<_> = proposal
        .commands
        .iter()
        .filter_map(|c| match c {
            Command::AddParameter { parameter } => Some(parameter.id.as_str().to_owned()),
            _ => None,
        })
        .collect();
    assert_eq!(added, ["AngleX", "AngleY"]);
    let applied = apply(model, &proposal);
    assert!(ugoe_model::validate::validate(&applied).is_empty());
    let right = face_points(&applied, &[("AngleX", 30.0)]);
    let base = face_points(&applied, &[]);
    assert!(right[at(2, 2)].x > base[at(2, 2)].x + 15.0);
}

#[test]
fn authored_cells_are_kept() {
    // 先に AngleX = 30 を手で作っておくと、提案はそのセルを上書きしない。
    let model = face_model();
    let mut doc = Document::new(model.clone());
    for key in [0.0, 30.0] {
        doc.apply(Command::AddKeyformKey {
            target: ugoe_editor_core::KeyformTarget::Deformer("Face".into()),
            parameter: "AngleX".into(),
            key,
        })
        .unwrap();
    }
    let prepared = doc.model().clone();
    let proposal = propose(&prepared);
    let applied = apply(prepared, &proposal);
    let DeformerKind::Warp(warp) = &applied.deformers[0].kind else {
        panic!("Face is not a warp");
    };
    let grid = &warp.keyforms;
    let keys = &grid.bindings[0].keys;
    assert_eq!(keys, &[-30.0, 0.0, 30.0]);
    // AngleX = 30, AngleY = 0 のセル (添字 2 + 3·1) は手で作ったまま (基準と同じ形)。
    assert_eq!(grid.origin(5), KeyformOrigin::Authored);
    assert_eq!(grid.keyforms[5], grid.keyforms[4]);
    // 足した AngleX = -30 は推定で作る。
    assert_eq!(grid.origin(3), KeyformOrigin::DepthRotation);
    assert!(ugoe_model::validate::validate(&applied).is_empty());
}

#[test]
fn server_errors_and_bad_maps_are_reported() {
    let model = face_model();
    let cassette = serde_json::json!({
        "interactions": [
            { "request": { "method": "POST", "url": ENDPOINT },
              "response": { "status": 503, "body_text": "model is loading" } },
            { "request": { "method": "POST", "url": ENDPOINT },
              "response": { "status": 200, "body": { "width": 2, "height": 2, "depth": [1, 2, 3] } } },
        ]
    });
    // 再試行しない設定で、503 をそのまま返すことを確かめる (再試行は下のテスト)
    let config = HttpDepthConfig {
        retry: RetryPolicy {
            max_retries: 0,
            ..Default::default()
        },
        ..HttpDepthConfig::new(ENDPOINT, "m")
    };
    let estimator = HttpDepthEstimator::with_transport(
        config,
        ReplayTransport::new(serde_json::from_value(cassette).unwrap()),
    );
    let request = DepthRequest { image: png() };
    let run = || propose_depth_turn(&model, &estimator, &request, &face_labels(), &[face()]);
    assert!(matches!(run(), Err(AiError::Api { status: 503, .. })));
    assert!(matches!(run(), Err(AiError::InvalidResponse(_))));

    // PNG でなければ送らない。
    let not_png = DepthRequest {
        image: b"GIF89a".to_vec(),
    };
    assert!(matches!(
        estimator.estimate(&not_png),
        Err(AiError::InvalidRequest(_))
    ));
}

#[test]
fn targets_must_be_warps_with_known_roles() {
    let model = face_model();
    let flat = FixedDepthEstimator::new(DepthMap::new(1, 1, vec![1.0]).unwrap()).unwrap();
    let request = DepthRequest { image: png() };
    let missing = TurnTarget {
        deformer: "Nope".into(),
        role: None,
    };
    assert!(matches!(
        propose_depth_turn(&model, &flat, &request, &face_labels(), &[missing]),
        Err(AiError::InvalidRequest(_))
    ));
    // ラベルがなく部位も指定しなければ、頭か体か分からない。
    assert!(matches!(
        propose_depth_turn(&model, &flat, &request, &Default::default(), &[face()]),
        Err(AiError::InvalidRequest(_))
    ));
    // 一様な深度なら顔の制御点はすべて輪郭と同じ深さ (平らな板) になる。
    let proposal = propose_depth_turn(&model, &flat, &request, &face_labels(), &[face()]).unwrap();
    let depth = depth_of(&proposal);
    assert_eq!(depth[at(2, 2)], 0.0);
}

#[test]
fn disclosure_depends_on_the_endpoint() {
    let request = DepthRequest { image: png() };
    let local = estimator().disclosure(&request);
    assert_eq!(local.destination, Destination::Local);
    assert_eq!(local.image_count, 1);
    assert_eq!(local.image_bytes, png().len());
    let remote = HttpDepthEstimator::with_transport(
        HttpDepthConfig::new("https://depth.example.com/v1", "depth-anything-v2-large"),
        ReplayTransport::new(Default::default()),
    )
    .disclosure(&request);
    assert!(remote.is_external());
}

#[test]
fn layer_labels_are_matched_to_meshes_by_name() {
    let model = face_model();
    let layer = |path: &[&str], label| LayerLabel {
        layer_path: path.iter().map(|s| s.to_string()).collect(),
        label,
        side: Side::Center,
        confidence: 0.9,
        variant: None,
    };
    let layers = [
        layer(&["Head", "Skin"], Label::Face),
        layer(&["Head", "Bangs"], Label::HairFront),
        // 同じ名前で違うラベルのレイヤーがあれば決めない。
        layer(&["Head", "Nose"], Label::Nose),
        layer(&["Other", "Nose"], Label::Other),
        // メッシュのない名前は無視する。
        layer(&["Head", "Hat"], Label::Headwear),
    ];
    let labels = mesh_labels(&model, &layers);
    let expected: Vec<(String, Label)> = vec![
        ("Bangs".into(), Label::HairFront),
        ("Skin".into(), Label::Face),
    ];
    let actual: Vec<(String, Label)> = labels
        .into_iter()
        .map(|(m, l)| (m.as_str().to_owned(), l))
        .collect();
    assert_eq!(actual, expected);

    // 同じ名前のメッシュが 2 つあれば、どちらにも付けない。
    let mut twins = model.clone();
    let mut copy = twins
        .meshes
        .iter()
        .find(|m| m.name == "Bangs")
        .unwrap()
        .clone();
    copy.id = "Bangs2".into();
    twins.meshes.push(copy);
    let labels = mesh_labels(&twins, &layers);
    assert_eq!(labels.len(), 1, "{labels:?}");
    assert!(labels.keys().all(|m| m.as_str() != "Bangs"));
}

const SECRET: &str = "depth-secret-key";
const MAP_2X2: &str = r#"{ "width": 2, "height": 2, "depth": [1, 2, 3, 4] }"#;

fn replay(interactions: serde_json::Value) -> Arc<ReplayTransport> {
    let cassette: Cassette =
        serde_json::from_value(serde_json::json!({ "interactions": interactions })).unwrap();
    Arc::new(ReplayTransport::new(cassette))
}

/// 待った時間を記録する推定器。
fn retrying(
    config: HttpDepthConfig,
    transport: Arc<ReplayTransport>,
) -> (HttpDepthEstimator, Arc<Mutex<Vec<Duration>>>) {
    let slept = Arc::new(Mutex::new(Vec::new()));
    let log = slept.clone();
    let estimator = HttpDepthEstimator::with_transport(config, transport)
        .with_sleeper(move |d| log.lock().unwrap().push(d));
    (estimator, slept)
}

fn assert_no_secret(e: &AiError) {
    assert!(!e.to_string().contains(SECRET), "{e}");
    assert!(!format!("{e:?}").contains(SECRET), "{e:?}");
}

#[test]
fn retries_connection_failures_and_unavailable_servers() {
    let map: serde_json::Value = serde_json::from_str(MAP_2X2).unwrap();
    let transport = replay(serde_json::json!([
        // 推論サーバの起動直後: 接続できない → モデルを読み込み中
        { "request": { "method": "POST", "url": ENDPOINT },
          "error": { "kind": "connect", "message": "connection refused" } },
        { "request": { "method": "POST", "url": ENDPOINT },
          "response": { "status": 503, "headers": { "retry-after": "3" }, "body_text": "loading" } },
        { "request": { "method": "POST", "url": ENDPOINT },
          "response": { "status": 200, "body": map } },
    ]));
    let (estimator, slept) = retrying(HttpDepthConfig::new(ENDPOINT, "m"), transport.clone());
    let depth = estimator.estimate(&DepthRequest { image: png() }).unwrap();
    assert_eq!(depth.values, vec![1.0, 2.0, 3.0, 4.0]);
    // 1 回目はバックオフ、2 回目は retry-after に従う
    assert_eq!(
        *slept.lock().unwrap(),
        vec![Duration::from_millis(500), Duration::from_secs(3)]
    );
    assert_eq!(transport.remaining(), 0);
}

#[test]
fn gives_up_after_the_retry_limit_and_keeps_the_failure_kind() {
    let timeout = serde_json::json!({ "request": { "method": "POST", "url": ENDPOINT },
        "error": { "kind": "timeout", "message": "timed out reading response" } });
    let transport = replay(serde_json::json!([timeout, timeout, timeout]));
    let (estimator, slept) = retrying(HttpDepthConfig::new(ENDPOINT, "m"), transport.clone());
    let e = estimator
        .estimate(&DepthRequest { image: png() })
        .unwrap_err();
    // プロバイダと同じく、種類 (Timeout) を文言に残す
    assert!(
        matches!(&e, AiError::Network(m) if m.starts_with("Timeout:") && m.contains("timed out")),
        "{e:?}"
    );
    assert_eq!(slept.lock().unwrap().len(), 2);
    assert_eq!(transport.remaining(), 0);

    // 再試行しない失敗 (4xx・それ以外の通信の失敗) は 1 回で諦める
    let transport = replay(serde_json::json!([
        { "request": { "method": "POST", "url": ENDPOINT },
          "response": { "status": 400, "body_text": "bad image" } },
    ]));
    let (estimator, slept) = retrying(HttpDepthConfig::new(ENDPOINT, "m"), transport);
    let e = estimator
        .estimate(&DepthRequest { image: png() })
        .unwrap_err();
    assert!(
        matches!(&e, AiError::Api { status: 400, kind, message } if kind == "depth_server_error" && message == "bad image"),
        "{e:?}"
    );
    assert!(slept.lock().unwrap().is_empty());

    // 429 はプロバイダと同じくレート制限として返す
    let transport = replay(serde_json::json!([
        { "request": { "method": "POST", "url": ENDPOINT },
          "response": { "status": 429, "headers": { "retry-after": "600" } } },
    ]));
    let (estimator, _) = retrying(HttpDepthConfig::new(ENDPOINT, "m"), transport);
    assert!(matches!(
        estimator.estimate(&DepthRequest { image: png() }),
        Err(AiError::RateLimited { retry_after: Some(d) }) if d == Duration::from_secs(600)
    ));
}

#[test]
fn sends_the_api_key_as_a_bearer_token_without_leaking_it() {
    const REMOTE: &str = "https://depth.example.com/v1";
    let map: serde_json::Value = serde_json::from_str(MAP_2X2).unwrap();
    let transport = replay(serde_json::json!([
        { "request": { "method": "POST", "url": REMOTE },
          "response": { "status": 200, "body": map } },
        { "request": { "method": "POST", "url": REMOTE },
          "response": { "status": 401, "body_text": format!("invalid key {SECRET}") } },
        { "request": { "method": "POST", "url": REMOTE },
          "error": { "kind": "connect", "message": format!("refused for {SECRET}") } },
    ]));
    let config = HttpDepthConfig {
        retry: RetryPolicy {
            max_retries: 0,
            ..Default::default()
        },
        ..HttpDepthConfig::new(REMOTE, "m")
    };
    let recording = Arc::new(RecordingTransport::new(transport.clone()));
    let estimator = HttpDepthEstimator::with_transport(config, recording.clone())
        .with_api_key(ApiKey::new(SECRET).unwrap());
    let request = DepthRequest { image: png() };
    estimator.estimate(&request).unwrap();

    // ヘッダは送るが、受け取ったリクエストの記録では伏せてある
    let received = transport.received();
    let auth = received[0]
        .headers
        .iter()
        .find(|(k, _)| k == "authorization")
        .map(|(_, v)| v.as_str());
    assert_eq!(auth, Some("<redacted>"));
    assert!(!format!("{received:?}").contains(SECRET));
    assert!(!format!("{estimator:?}").contains(SECRET));

    // エラーの文言に紛れ込んだ鍵も伏せる
    let e = estimator.estimate(&request).unwrap_err();
    assert!(matches!(&e, AiError::Api { status: 401, .. }), "{e:?}");
    assert!(e.to_string().contains("<redacted>"));
    assert_no_secret(&e);
    let e = estimator.estimate(&request).unwrap_err();
    assert!(
        matches!(&e, AiError::Network(m) if m.starts_with("Connect:")),
        "{e:?}"
    );
    assert_no_secret(&e);

    // 録画したリクエストにも残らない (応答はサーバが返したまま録る)
    let cassette = recording.cassette();
    assert_eq!(cassette.interactions.len(), 3);
    for interaction in &cassette.interactions {
        let request = serde_json::to_string(&interaction.request).unwrap();
        assert!(!request.contains(SECRET), "{request}");
    }
}

#[test]
fn sends_no_authorization_header_without_a_key() {
    let map: serde_json::Value = serde_json::from_str(MAP_2X2).unwrap();
    let transport = replay(serde_json::json!([
        { "request": { "method": "POST", "url": ENDPOINT },
          "response": { "status": 200, "body": map } },
    ]));
    let estimator =
        HttpDepthEstimator::with_transport(HttpDepthConfig::new(ENDPOINT, "m"), transport.clone());
    estimator.estimate(&DepthRequest { image: png() }).unwrap();
    let headers = &transport.received()[0].headers;
    assert!(
        headers.iter().all(|(k, _)| k != "authorization"),
        "{headers:?}"
    );
}
