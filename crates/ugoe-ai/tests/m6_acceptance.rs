//! M6 (AI 支援) の受け入れテスト (T0510)。ネットワークは使わない。
//!
//! 完了の条件「PSD から、顔の角度・目・口・呼吸・髪揺れが動くモデルを、提案を採用するだけで作れる」を、
//! 胸から上の実際の PSD (`assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd`) で確かめる。
//!
//! 1. 分類: 録画 (`tests/cassettes/classify_layers_bustup.json`、T0503 の手で作った Messages API の応答) を
//!    [`ClaudeProvider`] で再生する。
//! 2. リギングの提案: 録画 (`tests/cassettes/m6_rig_bustup.json`) を再生する。答えの値は、T0510 で Claude Code が
//!    MCP (`ugoe-mcp`) の経路で実際に画像を見て決めた値 (`docs/tasks/T0510-ai-acceptance.md` のメモ) を、
//!    Messages API の応答の形にしたもの。
//! 3. 採用: エディタと同じ [`Proposals`] に置き、1 回で採用する (1 回の Undo で戻る)。
//! 4. アバタープロファイル: 利用者がメタデータ (名前・作者) を書き、レベル 1 を名乗る。
//! 5. ランタイム: `.ugoe` に書き出して読み直し、[`Puppet`] で顔の角度・目・口・呼吸・髪揺れが動くことを数値で確かめる。

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ugoe::{Puppet, Toggles};
use ugoe_ai::ApiKey;
use ugoe_ai::classify::{ClassifyOptions, classify_layers, overview_png};
use ugoe_ai::claude::{ClaudeConfig, ClaudeProvider};
use ugoe_ai::rig::{DEFAULT_MAX_ATTEMPTS, RigOptions, mesh_parts, propose_rig_with_ai};
use ugoe_ai::transport::{Cassette, ReplayTransport};
use ugoe_core::Vec2;
use ugoe_editor_core::{
    Command, Document, Project, ProjectMeta, Proposals, PsdImportOptions, export_bundle, import_psd,
};
use ugoe_model::avatar::AvatarMeta;
use ugoe_model::avatar::check::check_profile;
use ugoe_model::physics::PhysicsTarget;
use ugoe_model::standard;

const PSD: &str = "comfyui-see-through-bustup-parts.psd";
const DT: f32 = 1.0 / 60.0;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn replay(cassette: &str) -> (ClaudeProvider, Arc<ReplayTransport>) {
    let cassette = Cassette::load(repo_path("tests/cassettes").join(cassette)).unwrap();
    let transport = Arc::new(ReplayTransport::new(cassette));
    let provider = ClaudeProvider::with_transport(
        ClaudeConfig::default(),
        ApiKey::new("sk-ant-test").unwrap(),
        transport.clone(),
    );
    (provider, transport)
}

/// PSD の取り込みから採用・プロファイルの設定までを行ったプロジェクト。
struct Adopted {
    doc: Document,
    /// 採用した提案の題と理由。
    title: String,
    rationale: String,
}

fn adopted() -> &'static Adopted {
    static ADOPTED: OnceLock<Adopted> = OnceLock::new();
    ADOPTED.get_or_init(build)
}

fn build() -> Adopted {
    let path = repo_path("../../assets/fixtures/psd-real").join(PSD);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut doc = Project::empty("bustup").document();
    let import = import_psd(doc.model(), &bytes, &PsdImportOptions::new(PSD)).unwrap();
    doc.apply(import.command).unwrap();
    let tree = ugoe_psd::import(&bytes).unwrap();

    // 1. 分類 (録画の再生)。
    let (provider, transport) = replay("classify_layers_bustup.json");
    let labels = classify_layers(&provider, &tree, &ClassifyOptions::default()).unwrap();
    assert_eq!(transport.remaining(), 0);
    let parts = mesh_parts(doc.model(), &labels);
    assert_eq!(
        parts.len(),
        doc.model().meshes.len(),
        "every mesh is labeled"
    );

    // 2. リギングの提案 (録画の再生。1 回で通る)。
    let (provider, transport) = replay("m6_rig_bustup.json");
    let rig = propose_rig_with_ai(
        &provider,
        &doc,
        &parts,
        &overview_png(&tree, 1024).unwrap(),
        &RigOptions::default(),
        DEFAULT_MAX_ATTEMPTS,
    )
    .unwrap();
    assert_eq!(transport.remaining(), 0);
    assert!(rig.rejected.is_empty());
    assert_eq!(rig.params.turn_angle_scale, 0.6);
    assert_eq!(rig.params.eye_close_bend, -0.06);

    // 3. 採用: 提案を置き (複製で検証される)、そのまま 1 回で採用する。
    let mut proposals = Proposals::new();
    let view = proposals
        .add(
            &doc,
            rig.proposal.title.clone(),
            rig.proposal.rationale.clone(),
            rig.proposal.commands.clone(),
        )
        .unwrap();
    assert!(doc.model().parameters.is_empty());
    proposals.adopt(&mut doc, view.id, &[]).unwrap();
    let errors = ugoe_model::validate::validate(doc.model());
    assert!(errors.is_empty(), "{errors:?}");
    // 1 回の Undo で提案の前に戻り、Redo で戻る。
    assert!(doc.undo());
    assert!(doc.model().parameters.is_empty());
    assert!(doc.physics().is_none());
    assert!(doc.redo());

    // 4. アバタープロファイル: 名前と作者は利用者が書く (提案は作者を知らない)。
    doc.apply(Command::UpdateAvatarProfile {
        level: Some(1),
        meta: Some(AvatarMeta {
            name: "Bust-up sample".to_owned(),
            authors: vec!["ugoe".to_owned()],
            ..AvatarMeta::default()
        }),
        tracking: None,
        look_at: None,
        hit_areas: None,
        slots: None,
    })
    .unwrap();

    Adopted {
        doc,
        title: rig.proposal.title,
        rationale: rig.proposal.rationale,
    }
}

/// `.ugoe` に書き出して読み直したバンドルから作った `Puppet` (物理つき)。
fn puppet(toggles: Toggles) -> Puppet {
    let project = Project::from_document(ProjectMeta::new("bustup"), &adopted().doc);
    let bundle = ugoe::format::read_bundle(&export_bundle(&project).unwrap()).unwrap();
    let mut puppet = Puppet::new(bundle.model)
        .unwrap()
        .with_physics(bundle.physics.expect("the proposal adds physics"));
    puppet.set_toggles(toggles);
    puppet
}

/// 物理だけを有効にした段 (パラメータを手で与えて確かめる)。
fn physics_only() -> Toggles {
    Toggles {
        physics: true,
        eye_blink: false,
        breath: false,
        lip_sync: false,
        pose: false,
    }
}

fn set(puppet: &mut Puppet, id: &str, value: f32) {
    let params = puppet.instance_mut().parameters_mut();
    let index = params.index_of(id).unwrap();
    params.set(index, value);
}

fn get(puppet: &Puppet, id: &str) -> f32 {
    let params = puppet.instance().parameters();
    params.get(params.index_of(id).unwrap()).unwrap()
}

/// 名前 (PSD のレイヤー名) ごとのメッシュの頂点 (キャンバス座標) と不透明度。
fn meshes(puppet: &Puppet) -> BTreeMap<String, (Vec<Vec2>, f32)> {
    let doc = &adopted().doc;
    doc.model()
        .meshes
        .iter()
        .map(|m| {
            let i = puppet
                .instance()
                .compiled()
                .mesh_index(m.id.as_str())
                .unwrap();
            let d = &puppet.instance().drawables()[i];
            (m.name.clone(), (d.positions.clone(), d.opacity))
        })
        .collect()
}

fn center(points: &[Vec2]) -> Vec2 {
    points.iter().fold(Vec2::ZERO, |a, p| a + *p) / points.len() as f32
}

fn max_distance(a: &[Vec2], b: &[Vec2]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(p, q)| (*p - *q).length())
        .fold(0.0, f32::max)
}

#[test]
fn the_adopted_proposal_covers_every_part_and_the_profile_passes() {
    let adopted = adopted();
    let model = adopted.doc.model();
    assert!(adopted.title.contains("breath") && adopted.title.contains("hair sway"));
    for part in [
        "Head:",
        "Eye L:",
        "Eye R:",
        "Mouth:",
        "Breath:",
        "Hair sway:",
        "Effects:",
    ] {
        assert!(adopted.rationale.contains(part), "{part}");
    }
    for id in [
        standard::ANGLE_X,
        standard::ANGLE_Y,
        standard::ANGLE_Z,
        standard::EYE_L_OPEN,
        standard::EYE_R_OPEN,
        standard::MOUTH_OPEN_Y,
        standard::BREATH,
    ] {
        assert!(model.parameters.iter().any(|p| p.id == id), "{id}");
    }
    // エフェクト: 目パチ・呼吸・リップシンクが作ったパラメータに向いている。
    let effects = &model.effects;
    assert_eq!(effects.eye_blink.parameters.len(), 2);
    assert!(
        effects
            .breath
            .parameters
            .iter()
            .any(|b| b.parameter == standard::BREATH)
    );
    assert_eq!(effects.lip_sync.parameters, vec![standard::MOUTH_OPEN_Y]);
    // 髪揺れ: 前髪と後ろ髪の動的物理 (頭の回転デフォーマに取り付く)。
    let physics = adopted.doc.physics().unwrap();
    assert_eq!(physics.rigs.len(), 2);
    for rig in &physics.rigs {
        assert!(rig.frame.is_some(), "{}", rig.id);
        assert!(matches!(
            rig.outputs[0].target,
            PhysicsTarget::Vertices { .. }
        ));
    }

    // アバタープロファイル: レベル 1 (基本) を名乗り、準拠する。呼吸はあるが、レベル 2 に要る視線・眉・口の形・
    // 体の向きは提案が作らないので、満たすのはレベル 1 まで。
    let report = check_profile(model);
    let errors: Vec<_> = report.errors().collect();
    assert!(report.is_conformant(), "{errors:?}");
    assert_eq!(report.claimed, Some(1));
    assert_eq!(report.satisfied, 1);
    assert_eq!(report.warnings().count(), 0, "{:?}", report.issues);
}

#[test]
fn the_face_angles_eyes_and_mouth_move_at_runtime() {
    let mut puppet = puppet(Toggles {
        physics: false,
        ..physics_only()
    });
    puppet.update(DT);
    let base = meshes(&puppet);
    let nose = |m: &BTreeMap<String, (Vec<Vec2>, f32)>| center(&m["nose"].0);
    let pose = |puppet: &mut Puppet, values: &[(&str, f32)]| {
        for id in [
            standard::ANGLE_X,
            standard::ANGLE_Y,
            standard::ANGLE_Z,
            standard::EYE_L_OPEN,
            standard::EYE_R_OPEN,
            standard::MOUTH_OPEN_Y,
        ] {
            let default = if id.starts_with("Eye") { 1.0 } else { 0.0 };
            set(puppet, id, default);
        }
        for (id, v) in values {
            set(puppet, id, *v);
        }
        puppet.update(DT);
        meshes(puppet)
    };

    // 顔の角度: AngleX で鼻が向いた側 (画面の右) へ、AngleY で上へ、AngleZ で頭が傾き体は動かない。
    let right = pose(&mut puppet, &[(standard::ANGLE_X, 30.0)]);
    assert!(nose(&right).x - nose(&base).x > 5.0);
    let up = pose(&mut puppet, &[(standard::ANGLE_Y, 30.0)]);
    assert!(nose(&up).y < nose(&base).y - 3.0);
    let tilt = pose(&mut puppet, &[(standard::ANGLE_Z, 30.0)]);
    assert!((nose(&tilt) - nose(&base)).length() > 5.0);
    assert!(max_distance(&tilt["topwear"].0, &base["topwear"].0) < 0.01);

    // 目: 左目だけが閉じた目の絵に入れ替わる。
    let wink = pose(&mut puppet, &[(standard::EYE_L_OPEN, 0.0)]);
    assert_eq!(wink["eyeclose-l"].1, 1.0);
    assert_eq!(wink["eyelash-l"].1, 0.0);
    assert_eq!(wink["eyeclose-r"].1, 0.0);
    // 口: 開いた口の絵に入れ替わる。
    let open = pose(&mut puppet, &[(standard::MOUTH_OPEN_Y, 1.0)]);
    assert_eq!(open["mouth-open"].1, 1.0);
    assert_eq!(open["mouth-close"].1, 0.0);
    assert_eq!(base["mouth-open"].1, 0.0);
}

#[test]
fn the_effects_blink_breathe_and_lip_sync() {
    let mut puppet = puppet(Toggles {
        physics: false,
        eye_blink: true,
        breath: true,
        lip_sync: true,
        pose: false,
    });
    let (mut breath_min, mut breath_max) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut neck_min, mut neck_max) = (f32::INFINITY, f32::NEG_INFINITY);
    let mut eye_min = f32::INFINITY;
    for _ in 0..(10.0 / DT) as usize {
        puppet.update(DT);
        let breath = get(&puppet, standard::BREATH);
        breath_min = breath_min.min(breath);
        breath_max = breath_max.max(breath);
        eye_min = eye_min.min(get(&puppet, standard::EYE_L_OPEN));
        let neck = center(&meshes(&puppet)["neck"].0).y;
        neck_min = neck_min.min(neck);
        neck_max = neck_max.max(neck);
    }
    // 呼吸: `Breath` が 0〜1 を往復し、体 (首) が上下する。
    assert!(
        breath_min < 0.05 && breath_max > 0.95,
        "{breath_min}..{breath_max}"
    );
    assert!(neck_max - neck_min > 3.0, "{neck_min}..{neck_max}");
    // 目パチ: 10 秒のうちに目が閉じる。
    assert!(eye_min < 0.05, "{eye_min}");
    // リップシンク: 音量で口が開く。
    puppet.set_lip_sync_level(Some(1.0));
    puppet.update(DT);
    assert_eq!(get(&puppet, standard::MOUTH_OPEN_Y), 1.0);
    assert_eq!(meshes(&puppet)["mouth-open"].1, 1.0);
}

#[test]
fn the_hair_sways_behind_the_head() {
    let mut moving = puppet(physics_only());
    let mut still = puppet(Toggles {
        physics: false,
        ..physics_only()
    });
    // 静止: 頭が動かなければ、物理があっても髪は動かない。
    for _ in 0..60 {
        moving.update(DT);
        still.update(DT);
    }
    for hair in ["front hair", "back hair"] {
        let d = max_distance(&meshes(&moving)[hair].0, &meshes(&still)[hair].0);
        assert!(d < 0.5, "{hair} drifts {d}px at rest");
    }

    // 頭を急に向けると、髪 (毛先) が遅れて揺れる。生え際 (顔の上端の近く) はほとんど動かない。
    let mut peak: BTreeMap<&str, f32> = BTreeMap::new();
    let mut last: BTreeMap<&str, Vec<Vec2>> = BTreeMap::new();
    for frame in 0..(6.0 / DT) as usize {
        for p in [&mut moving, &mut still] {
            set(p, standard::ANGLE_X, 30.0);
            p.update(DT);
        }
        let (m, s) = (meshes(&moving), meshes(&still));
        for hair in ["front hair", "back hair"] {
            let d = max_distance(&m[hair].0, &s[hair].0);
            let e = peak.entry(hair).or_default();
            *e = e.max(d);
            if frame + 30 == (6.0 / DT) as usize {
                last.insert(hair, m[hair].0.clone());
            }
        }
        // 顔の肌は物理の対象でない。
        assert!(max_distance(&m["face"].0, &s["face"].0) < 1e-3);
    }
    let front = peak["front hair"];
    let back = peak["back hair"];
    assert!(front > 2.0, "front hair sways {front}px");
    assert!(
        back > 2.0 * front,
        "back hair sways {back}px (front {front}px)"
    );
    // 6 秒で揺れは収まる (最後の 0.5 秒で動かない。後ろ髪は長くゆっくり揺れる)。
    let now = meshes(&moving);
    for hair in ["front hair", "back hair"] {
        let d = max_distance(&now[hair].0, &last[hair]);
        assert!(d < 0.5, "{hair} still moves {d}px");
    }
}
