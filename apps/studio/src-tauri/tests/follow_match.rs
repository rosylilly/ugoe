//! 一致の検査 (ADR 0028 の 10.): 指揮役が配ったフレームの列を記録し、バイト列を通して受け手に順に適用すると、
//! 物理を有効にした 1 つの `Puppet` と同じ形 (頂点とパーツの不透明度) になる。
//!
//! 入力の列は、パラメータの書き換え・揃っていない `dt`・リップシンクの音量を混ぜ、途中でモデルの読み直しと物理のリセットを含める。
//! リップシンクの対象は物理の入力 (`AngleX`) にする (受け手が物理の後にリップシンクを当てることの確かめ。T0134)。
//! 受け手の描画の頻度に依らないこと (まとめて適用してから比べても同じ) も確かめる。
#![allow(clippy::unwrap_used)]

use ugoe::Puppet;
use ugoe::format::Bundle;
use ugoe::model::effects::{Breath, BreathParameter, LipSync};
use ugoe::model::physics::PhysicsSettings;
use ugoe::model::{Model, fixtures};
use ugoe_studio_lib::conductor::Conductor;
use ugoe_studio_lib::follower::{Follower, Received};
use ugoe_studio_lib::stream::Frame;

const EPSILON: f32 = 1e-3;

/// 比べる状態: パラメータの値 (物理の出力を含む) と、描画物ごとの頂点の座標と不透明度。
fn shape(puppet: &Puppet) -> Vec<f32> {
    let values = puppet.instance().parameters().as_slice().to_vec();
    let drawables = puppet.instance().drawables().iter().flat_map(|d| {
        d.positions
            .iter()
            .flat_map(|p| [p.x, p.y])
            .chain([d.opacity])
            .collect::<Vec<_>>()
    });
    values.into_iter().chain(drawables).collect()
}

fn assert_close(a: &[f32], b: &[f32], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}");
    for (i, (a, b)) in a.iter().zip(b).enumerate() {
        assert!((a - b).abs() <= EPSILON, "{what} [{i}]: {a} != {b}");
    }
}

fn bundle(model: &Model, physics: Option<&PhysicsSettings>) -> Bundle {
    let mut bundle = Bundle::new(model.clone());
    bundle.physics = physics.cloned();
    bundle
}

/// 物理を有効にした 1 つの `Puppet` (比べる相手)。
fn single(bundle: &Bundle) -> Puppet {
    let puppet = Puppet::new(bundle.model.clone()).unwrap();
    match &bundle.physics {
        Some(physics) => puppet.with_physics(physics.clone()),
        None => puppet,
    }
}

/// 揃っていない `dt` の列 (60 fps のゆらぎ・落ちたフレーム・止まっていた後の大きな値)。
fn dt_at(frame: usize) -> f32 {
    match frame % 7 {
        0 => 1.0 / 30.0,
        3 => 0.013,
        5 => 0.021,
        _ => 1.0 / 60.0,
    }
}

/// 呼吸とリップシンクを `input` に当てるモデル (どちらも指揮役で入力を作る)。
fn with_effects(mut model: Model, input: &str) -> Model {
    model.effects.breath = Breath {
        parameters: vec![BreathParameter {
            parameter: input.into(),
            offset: 0.0,
            peak: 5.0,
            cycle: 1.5,
            weight: 1.0,
        }],
    };
    model.effects.lip_sync = LipSync {
        parameters: vec![input.into()],
    };
    model
}

fn run_case(name: &str, model: Model, physics: Option<PhysicsSettings>, input: &str) {
    let model = with_effects(model, input);
    let source = bundle(&model, physics.as_ref());
    let mut conductor = Conductor::new();
    let mut reference = single(&source);
    let mut follower = Follower::new();
    // 記録したフレームの列 (バイト列) と、各フレームの後の比べる相手の形。
    let mut recorded: Vec<(Vec<u8>, Vec<f32>)> = Vec::new();
    let mut scenes: Vec<(u32, usize)> = Vec::new();

    let mut load = |conductor: &mut Conductor, at: usize| {
        let revision = conductor.load(name, source.clone()).unwrap().revision;
        scenes.push((revision, at));
    };
    load(&mut conductor, 0);
    for frame in 0..240 {
        // 入力: 頭を振る。途中で書き換えを止めて既定値に戻す。
        let value = if frame < 180 {
            30.0 * (frame as f32 * 0.11).sin()
        } else {
            0.0
        };
        if frame == 90 {
            // 着せ替えのように、モデルを読み直さずに物理だけをリセットする。
            conductor.reset_physics();
            reference.reset_physics();
        }
        if frame == 150 {
            // モデルの読み直し (新しいシーン。物理は新しく始まり、値は既定値から)。
            load(&mut conductor, recorded.len());
            reference = single(&source);
        }
        assert!(conductor.set_parameter(input, value));
        let params = reference.instance_mut().parameters_mut();
        let index = params.index_of(input).unwrap();
        params.set(index, value);
        // 音量を揺らし、途中で止める (None)。
        let level = (frame < 200).then(|| (frame % 7) as f32 / 6.0);
        conductor.set_lip_sync_level(level);
        reference.set_lip_sync_level(level);
        let dt = dt_at(frame);
        let sent = conductor.tick(dt).unwrap();
        assert_eq!(
            sent.lip_sync.is_some(),
            level.is_some(),
            "{name} frame {frame}"
        );
        reference.update(dt);
        recorded.push((sent.encode(), shape(&reference)));
    }

    // (1) 1 フレームずつ適用し、各フレームの後で比べる。
    let mut next_scene = 0;
    for (i, (bytes, expected)) in recorded.iter().enumerate() {
        let frame = Frame::decode(bytes).unwrap();
        if let Some(&(revision, at)) = scenes.get(next_scene)
            && at == i
        {
            follower.set_scene(revision, single(&source));
            next_scene += 1;
        }
        assert_eq!(
            follower.receive(&frame),
            Ok(Received::Applied),
            "{name} frame {i}"
        );
        assert_close(
            &shape(follower.puppet().unwrap()),
            expected,
            &format!("{name} frame {i}"),
        );
    }

    // (2) 描画の頻度に依らない: 2 つ目のシーンの全フレームをまとめて適用してから描いても、最後の形は同じ。
    let (revision, at) = scenes[1];
    let mut batched = Follower::new();
    batched.set_scene(revision, single(&source));
    for (bytes, _) in &recorded[at..] {
        batched.receive(&Frame::decode(bytes).unwrap()).unwrap();
    }
    assert_eq!(
        shape(batched.puppet().unwrap()),
        shape(follower.puppet().unwrap()),
        "{name}: batched"
    );
}

/// 物理 (パラメータに出す物理)・動的物理・ポーズ (パーツの不透明度) と目パチ・呼吸のモデルで確かめる。
#[test]
fn followers_match_a_single_puppet_with_physics() {
    run_case(
        "physics_hair",
        fixtures::physics_hair(),
        Some(fixtures::physics_hair_settings()),
        "AngleX",
    );
    run_case(
        "dynamic_hair",
        fixtures::dynamic_hair(),
        Some(fixtures::dynamic_hair_settings()),
        "AngleX",
    );
    let pose = fixtures::pose_and_effects();
    let input = pose.parameters[0].id.to_string();
    run_case("pose_and_effects", pose, None, &input);
}

/// 物理が実際に効いている (受け手の形が、物理を切った形と違う)。上の検査が空振りしていないことの確認。
#[test]
fn physics_actually_moves_the_follower() {
    let source = bundle(
        &fixtures::physics_hair(),
        Some(&fixtures::physics_hair_settings()),
    );
    let mut conductor = Conductor::new();
    let revision = conductor.load("hair", source.clone()).unwrap().revision;
    let mut follower = Follower::new();
    follower.set_scene(revision, single(&source));
    let mut still = Follower::new();
    let mut no_physics = Puppet::new(source.model.clone()).unwrap();
    no_physics.set_toggles(ugoe::Toggles {
        physics: false,
        ..ugoe::Toggles::default()
    });
    still.set_scene(revision, no_physics);
    for frame in 0..60 {
        conductor.set_parameter("AngleX", if frame < 30 { 30.0 } else { -30.0 });
        let sent = conductor.tick(1.0 / 60.0).unwrap();
        follower.receive(&sent).unwrap();
        still.receive(&sent).unwrap();
    }
    assert_ne!(
        shape(follower.puppet().unwrap()),
        shape(still.puppet().unwrap())
    );
}
