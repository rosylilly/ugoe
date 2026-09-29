//! 自動リギングの提案 (`ugoe_ai::rig`) のテスト。ネットワークは使わない。
//!
//! 実際の PSD (`assets/fixtures/psd-real/`) を取り込み、レイヤー名の規則で分類して提案を作り、適用・検証して、
//! ugoe-runtime で評価した見た目 (頂点と不透明度) を確かめる。AI の経路は、録画
//! (`tests/cassettes/auto_rig_retry.json`、Messages API の応答の形に合わせて手で作ったもの。
//! 1 回目は範囲の外の値、2 回目は正しい値) の再生と [`FakeProvider`] で確かめる。

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use serde_json::json;
use ugoe_ai::classify::{classify_by_name, overview_png};
use ugoe_ai::claude::{ApiKey, ClaudeConfig, ClaudeProvider};
use ugoe_ai::depth::DepthMap;
use ugoe_ai::rig::{
    MeshPart, RigOptions, RigParams, TurnDepth, mesh_parts, propose_rig, propose_rig_with_ai,
};
use ugoe_ai::transport::{Cassette, ReplayTransport};
use ugoe_ai::{AiError, FakeProvider, Proposal};
use ugoe_core::Vec2;
use ugoe_editor_core::{Command, Document, PsdImportOptions, import_psd};
use ugoe_model::{DeformerKind, KeyformInference, MeshId, Model, standard};
use ugoe_runtime::{CompiledModel, ModelInstance};

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// 取り込んだ PSD と、名前の規則による分類と、原画の PNG。
struct Input {
    model: Model,
    parts: BTreeMap<MeshId, MeshPart>,
    image: Vec<u8>,
}

fn load(name: &str) -> Input {
    let path = repo_path("../../assets/fixtures/psd-real").join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut doc = Document::new(ugoe_editor_core::Project::empty(name).model);
    let import = import_psd(doc.model(), &bytes, &PsdImportOptions::new(name)).unwrap();
    doc.apply(import.command).unwrap();
    let tree = ugoe_psd::import(&bytes).unwrap();
    let parts = mesh_parts(doc.model(), &classify_by_name(&tree));
    Input {
        model: doc.model().clone(),
        parts,
        image: overview_png(&tree, 256).unwrap(),
    }
}

fn bustup() -> &'static Input {
    static INPUT: OnceLock<Input> = OnceLock::new();
    INPUT.get_or_init(|| load("comfyui-see-through-bustup-parts.psd"))
}

fn full_body() -> &'static Input {
    static INPUT: OnceLock<Input> = OnceLock::new();
    INPUT.get_or_init(|| load("comfyui-see-through-full-body-parts.psd"))
}

/// 提案を `Batch` として適用し、検証を通ることを確かめる。
fn apply(model: &Model, proposal: &Proposal) -> Model {
    let mut doc = Document::new(model.clone());
    doc.apply(Command::Batch {
        label: proposal.title.clone(),
        commands: proposal.commands.clone(),
    })
    .unwrap();
    let errors = ugoe_model::validate::validate(doc.model());
    assert!(errors.is_empty(), "{errors:?}");
    doc.model().clone()
}

/// パラメータを `values` にして評価した、名前ごとのメッシュの頂点 (キャンバス座標) と不透明度。
fn evaluate(model: &Model, values: &[(&str, f32)]) -> BTreeMap<String, (Vec<Vec2>, f32)> {
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    for (id, v) in values {
        let index = instance.parameters().index_of(id).unwrap();
        instance.parameters_mut().set(index, *v);
    }
    instance.update();
    model
        .meshes
        .iter()
        .map(|m| {
            let d = &instance.drawables()[instance.compiled().mesh_index(m.id.as_str()).unwrap()];
            (m.name.clone(), (d.positions.clone(), d.opacity))
        })
        .collect()
}

fn center(points: &[Vec2]) -> Vec2 {
    points.iter().fold(Vec2::ZERO, |a, p| a + *p) / points.len() as f32
}

fn height(points: &[Vec2]) -> f32 {
    let (lo, hi) = points
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.y), hi.max(p.y))
        });
    hi - lo
}

fn angle_scale(model: &Model) -> f32 {
    model
        .deformers
        .iter()
        .find_map(|d| match &d.kind {
            DeformerKind::Warp(w) => match &w.keyforms.inference {
                Some(KeyformInference::DepthRotation(r)) => Some(r.angle_scale),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}

#[test]
fn bustup_rig_turns_the_head_and_opens_the_eyes_and_mouth() {
    let input = bustup();
    let proposal = propose_rig(
        &input.model,
        &input.parts,
        &RigParams::default(),
        &RigOptions::default(),
    )
    .unwrap();
    let model = apply(&input.model, &proposal);
    for id in [
        standard::ANGLE_X,
        standard::ANGLE_Y,
        standard::ANGLE_Z,
        standard::EYE_L_OPEN,
        standard::EYE_R_OPEN,
        standard::MOUTH_OPEN_Y,
    ] {
        assert!(model.parameters.iter().any(|p| p.id.as_str() == id), "{id}");
    }
    assert!((angle_scale(&model) - 0.7).abs() < 1e-6);

    let base = evaluate(&model, &[]);
    let before = evaluate(&input.model, &[]);
    // 既定値では見た目を変えない (閉じた目・開いた口は透明になり、他は元の位置)。
    // 開いた口は透明な細い隙間になる。
    for (name, (points, opacity)) in &before {
        let (after, after_opacity) = &base[name];
        if name == "mouth-open" {
            assert_eq!(*after_opacity, 0.0);
            continue;
        }
        let moved = points
            .iter()
            .zip(after)
            .map(|(a, b)| (*a - *b).length())
            .fold(0.0f32, f32::max);
        assert!(moved < 0.05, "{name} moved {moved}px");
        let expected = match name.as_str() {
            "eyeclose-l" | "eyeclose-r" | "mouth-open" => 0.0,
            _ => *opacity,
        };
        assert!((after_opacity - expected).abs() < 1e-4, "{name}");
    }

    // 目: 左目 (キャラクターの左) だけが閉じた目の絵に入れ替わる。
    let closed = evaluate(&model, &[(standard::EYE_L_OPEN, 0.0)]);
    assert_eq!(closed["eyewhite-l"].1, 0.0);
    assert_eq!(closed["eyelash-l"].1, 0.0);
    assert_eq!(closed["eyeclose-l"].1, 1.0);
    assert_eq!(closed["eyewhite-r"].1, 1.0);
    assert_eq!(closed["eyeclose-r"].1, 0.0);
    // 入れ替わる前に、開いた目は細くつぶれる。
    let squashed = evaluate(&model, &[(standard::EYE_L_OPEN, 0.35)]);
    let ratio = height(&squashed["eyelash-l"].0) / height(&base["eyelash-l"].0);
    assert!(ratio < 0.3, "{ratio}");
    assert_eq!(squashed["eyelash-l"].1, 1.0);
    // どの値でも、開いた目か閉じた目のどちらかが不透明。
    for i in 0..=100 {
        let v = i as f32 / 100.0;
        let e = evaluate(&model, &[(standard::EYE_R_OPEN, v)]);
        assert!(e["eyelash-r"].1.max(e["eyeclose-r"].1) > 0.99, "{v}");
    }

    // 口: 開いた口が細い隙間から開き、閉じた口と入れ替わる。
    let open = evaluate(&model, &[(standard::MOUTH_OPEN_Y, 1.0)]);
    assert_eq!(open["mouth-open"].1, 1.0);
    assert_eq!(open["mouth-close"].1, 0.0);
    let slit = evaluate(&model, &[(standard::MOUTH_OPEN_Y, 0.12)]);
    assert_eq!(slit["mouth-open"].1, 1.0);
    assert!(height(&slit["mouth-open"].0) < 0.5 * height(&open["mouth-open"].0));

    // 顔の向き: AngleX = 30 で鼻が画面の右 (キャラクターの左) へ、後ろ髪は鼻より動かない (逆へ回り込む)。
    let turned = evaluate(&model, &[(standard::ANGLE_X, 30.0)]);
    let nose = center(&turned["nose"].0).x - center(&base["nose"].0).x;
    let back = center(&turned["back hair"].0).x - center(&base["back hair"].0).x;
    assert!(nose > 5.0, "{nose}");
    assert!(back < nose - 5.0, "{back} vs {nose}");
    // AngleY = 30 で上を向く。
    let up = evaluate(&model, &[(standard::ANGLE_Y, 30.0)]);
    assert!(center(&up["nose"].0).y < center(&base["nose"].0).y - 3.0);
    // AngleZ で頭が傾く (首と体は動かない)。
    let tilted = evaluate(&model, &[(standard::ANGLE_Z, 30.0)]);
    assert!((center(&tilted["nose"].0) - center(&base["nose"].0)).length() > 5.0);
    assert!((center(&tilted["topwear"].0) - center(&base["topwear"].0)).length() < 0.01);
    assert!((center(&tilted["neck"].0) - center(&base["neck"].0)).length() < 0.01);
}

#[test]
fn full_body_rig_squashes_single_eyes_and_stretches_a_single_mouth() {
    let input = full_body();
    let proposal = propose_rig(
        &input.model,
        &input.parts,
        &RigParams::default(),
        &RigOptions::default(),
    )
    .unwrap();
    let model = apply(&input.model, &proposal);
    let base = evaluate(&model, &[]);

    // 閉じた目の絵がないので、開いた目をつぶして線にする (不透明度は変えない)。
    let closed = evaluate(&model, &[(standard::EYE_R_OPEN, 0.0)]);
    for part in ["eyelash-r", "eyewhite-r", "irides-r"] {
        let ratio = height(&closed[part].0) / height(&base[part].0);
        assert!(ratio < 0.1, "{part}: {ratio}");
        assert_eq!(closed[part].1, 1.0);
    }
    assert!((height(&closed["eyelash-l"].0) - height(&base["eyelash-l"].0)).abs() < 0.01);

    // 1 枚の口は縦に伸ばして開く。
    let open = evaluate(&model, &[(standard::MOUTH_OPEN_Y, 1.0)]);
    assert!(height(&open["mouth"].0) > height(&base["mouth"].0) + 2.0);
}

#[test]
fn an_estimated_depth_map_drives_the_turn() {
    let input = bustup();
    let depth_of = |proposal: &Proposal| {
        proposal
            .commands
            .iter()
            .find_map(|c| match c {
                Command::SetDepth { depth, .. } => depth.clone(),
                _ => None,
            })
            .unwrap()
    };
    let rules = propose_rig(
        &input.model,
        &input.parts,
        &RigParams::default(),
        &RigOptions::default(),
    )
    .unwrap();
    // 左ほど手前の傾いた板 (推定器が返したことにする)。
    let map = DepthMap::new(
        16,
        16,
        (0..256).map(|i| 1.0 - (i % 16) as f32 / 15.0).collect(),
    )
    .unwrap();
    let estimated = propose_rig(
        &input.model,
        &input.parts,
        &RigParams::default(),
        &RigOptions {
            turn: TurnDepth::Map(map),
            ..RigOptions::default()
        },
    )
    .unwrap();
    apply(&input.model, &estimated);
    assert_ne!(depth_of(&rules), depth_of(&estimated));
    assert!(estimated.rationale.contains("推定した深度マップ"));
}

#[test]
fn a_rejected_ai_answer_is_retried_with_the_errors() {
    let input = full_body();
    let cassette = Cassette::load(repo_path("tests/cassettes/auto_rig_retry.json")).unwrap();
    let transport = Arc::new(ReplayTransport::new(cassette));
    let provider = ClaudeProvider::with_transport(
        ClaudeConfig::default(),
        ApiKey::new("sk-ant-test").unwrap(),
        transport.clone(),
    );
    let rig = propose_rig_with_ai(
        &provider,
        &input.model,
        &input.parts,
        &input.image,
        &RigOptions::default(),
        3,
    )
    .unwrap();
    assert_eq!(transport.remaining(), 0);

    // 1 回目 (振り向きの係数 2.5) は範囲の外なので捨て、2 回目の値を使う。
    assert_eq!(rig.rejected.len(), 1);
    assert!(
        rig.rejected[0]
            .error
            .contains("turn_angle_scale is 2.5, but it must be between 0.3 and 1.2"),
        "{}",
        rig.rejected[0].error
    );
    assert_eq!(rig.params.turn_angle_scale, 0.6);
    assert_eq!(rig.params.eye_close_line, 0.72);
    let model = apply(&input.model, &rig.proposal);
    assert!((angle_scale(&model) - 0.6).abs() < 1e-6);
    assert!(rig.proposal.rationale.contains("small turn (0.6)"));

    // 2 回目のリクエストは、前の答えとエラーを添えて送る。
    let received = transport.received();
    assert_eq!(received.len(), 2);
    let text = |i: usize| received[i].body.as_ref().unwrap().to_string();
    assert!(!text(0).contains("Attempt 1"));
    assert!(text(1).contains("Attempt 1 was rejected"));
    assert!(text(1).contains("turn_angle_scale is 2.5"));
    // 構造化出力のスキーマと、原画 1 枚。
    let body = received[1].body.as_ref().unwrap();
    assert_eq!(
        body["output_config"]["format"]["schema"]["required"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    let images = body["messages"][0]["content"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["type"] == "image")
        .count();
    assert_eq!(images, 1);
}

fn answer(turn: f32) -> serde_json::Value {
    json!({
        "turn_angle_scale": turn,
        "eye_close_line": 0.7,
        "eye_close_bend": 0.04,
        "mouth_open_height": 0.35,
        "reason": "test"
    })
}

#[test]
fn the_ai_gives_up_after_the_attempt_limit() {
    let input = full_body();
    let fake = FakeProvider::new();
    fake.push(answer(5.0)).push(answer(-1.0)).push(answer(0.7));
    let error = propose_rig_with_ai(
        &fake,
        &input.model,
        &input.parts,
        &input.image,
        &RigOptions::default(),
        2,
    )
    .unwrap_err();
    assert!(
        matches!(&error, AiError::InvalidProposal(m) if m.contains("after 2 attempts") && m.contains("turn_angle_scale is -1")),
        "{error}"
    );
    assert_eq!(fake.calls().len(), 2);
}

#[test]
fn an_answer_that_does_not_match_the_schema_is_retried() {
    let input = full_body();
    let fake = FakeProvider::new();
    fake.push(json!({ "turn_angle_scale": 0.7 }))
        .push(answer(0.8));
    let rig = propose_rig_with_ai(
        &fake,
        &input.model,
        &input.parts,
        &input.image,
        &RigOptions::default(),
        3,
    )
    .unwrap();
    assert_eq!(rig.params.turn_angle_scale, 0.8);
    assert!(
        rig.rejected[0].error.contains("schema"),
        "{:?}",
        rig.rejected
    );
    assert!(
        fake.calls()[1]
            .0
            .prompt
            .contains("does not match the schema")
    );
}

#[test]
fn a_model_without_a_face_is_not_sent_to_the_ai() {
    let input = full_body();
    let parts: BTreeMap<MeshId, MeshPart> = input
        .parts
        .iter()
        .filter(|(_, p)| p.label != ugoe_ai::classify::Label::Face)
        .map(|(id, p)| (id.clone(), *p))
        .collect();
    let fake = FakeProvider::new();
    let error = propose_rig_with_ai(
        &fake,
        &input.model,
        &parts,
        &input.image,
        &RigOptions::default(),
        3,
    )
    .unwrap_err();
    assert!(matches!(error, AiError::InvalidRequest(_)), "{error}");
    assert!(fake.calls().is_empty());
}
