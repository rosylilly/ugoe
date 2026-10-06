//! 一致の検査 (ADR 0028 の 10.): 指揮役が配ったフレームの列を記録し、バイト列を通して受け手に順に適用すると、
//! 物理を有効にした 1 つの `Puppet` と同じ形 (頂点とパーツの不透明度) になる。
//!
//! 入力の列は、パラメータの書き換え・揃っていない `dt`・リップシンクの音量を混ぜ、途中でモデルの読み直しと物理のリセットを含める。
//! リップシンクの対象は物理の入力 (`AngleX`) にする (受け手が物理の後にリップシンクを当てることの確かめ。T0134)。
//! ポーズのモデルでは、指揮役がパーツを切り替えるモーションを再生する (受け手がモーションのパーツの不透明度のカーブを当てることの確かめ。T0135)。
//! 受け手の描画の頻度に依らないこと (まとめて適用してから比べても同じ) も確かめる。
//! シーンのポーズの要素を替えた出し直し (同じモデルのままはフェード、読み直しはフェードせずに) で、指揮役と受け手のパーツの
//! 不透明度が揃うことも確かめる (T0844)。
#![allow(clippy::unwrap_used)]

use ugoe::Puppet;
use ugoe::format::Bundle;
use ugoe::model::effects::{Breath, BreathParameter, LipSync};
use ugoe::model::motion::{Curve, CurveTarget, Motion, Point, Segment};
use ugoe::model::physics::PhysicsSettings;
use ugoe::model::{Model, fixtures};
use ugoe::motion::Priority;
use ugoe_studio_lib::conductor::{Conductor, NewModel, assemble};
use ugoe_studio_lib::follower::{self, Follower, Received};
use ugoe_studio_lib::stream::{Frame, Look, Scene};

const EPSILON: f32 = 1e-3;

/// 比べる状態: パラメータの値 (物理の出力を含む)、パーツの不透明度の上書き (値と重み)、描画物ごとの頂点の座標と不透明度。
fn shape(puppet: &Puppet) -> Vec<f32> {
    let instance = puppet.instance();
    let parts = (0..instance.compiled().model().parts.len()).flat_map(|i| {
        let (value, weight) = instance.part_opacity_override(i);
        [value, weight]
    });
    let values: Vec<f32> = instance
        .parameters()
        .as_slice()
        .iter()
        .copied()
        .chain(parts)
        .collect();
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

/// パーツを切り替えるモーション `switch` (`pose_and_effects` の `ArmA` から `ArmB` へ。`Root` の不透明度も下げる。フェードあり)。
fn switch_parts() -> Motion {
    let p = |t: f32, v: f32| Point { t, v };
    let curve = |part: &str, segments: Vec<Segment>| Curve {
        target: CurveTarget::PartOpacity(part.into()),
        segments,
        fade_in: None,
        fade_out: None,
    };
    Motion {
        name: "switch".into(),
        duration: 1.5,
        looping: false,
        fade_in: 0.3,
        fade_out: 0.4,
        curves: vec![
            curve(
                "ArmA",
                vec![
                    Segment::Start { p: p(0.0, 1.0) },
                    Segment::Stepped { to: p(0.5, 0.0) },
                ],
            ),
            curve(
                "ArmB",
                vec![
                    Segment::Start { p: p(0.0, 0.0) },
                    Segment::Stepped { to: p(0.5, 1.0) },
                ],
            ),
            curve(
                "Root",
                vec![
                    Segment::Start { p: p(0.0, 1.0) },
                    Segment::Linear { to: p(1.5, 0.3) },
                ],
            ),
        ],
        events: vec![],
    }
}

/// `motion` があれば、指揮役 (と比べる相手) はシーンごとに 10 フレーム目からそれを再生する。
fn run_case(
    name: &str,
    model: Model,
    physics: Option<PhysicsSettings>,
    input: &str,
    motion: Option<Motion>,
) {
    let model = with_effects(model, input);
    let mut source = bundle(&model, physics.as_ref());
    source.motions.extend(motion.clone());
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
        if let Some(motion) = motion.as_ref().filter(|_| frame == 10 || frame == 160) {
            let played = conductor.perform(|animation, puppet| {
                animation.play(puppet.unwrap(), &motion.name, Priority::Normal)
            });
            assert_eq!(played, Ok(true));
            assert!(reference.start_motion(motion.clone(), Priority::Normal));
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
        None,
    );
    run_case(
        "dynamic_hair",
        fixtures::dynamic_hair(),
        Some(fixtures::dynamic_hair_settings()),
        "AngleX",
        None,
    );
    let pose = fixtures::pose_and_effects();
    let input = pose.parameters[0].id.to_string();
    run_case("pose_and_effects", pose, None, &input, Some(switch_parts()));
}

/// パーツの不透明度のカーブが実際に配られている (受け手のポーズが切り替わる)。上の検査が空振りしていないことの確認 (T0135)。
#[test]
fn part_opacity_curves_reach_the_follower() {
    let mut source = bundle(&fixtures::pose_and_effects(), None);
    source.motions.push(switch_parts());
    let mut conductor = Conductor::new();
    let revision = conductor.load("pose", source.clone()).unwrap().revision;
    let mut follower = Follower::new();
    follower.set_scene(revision, single(&source));
    let played = conductor
        .perform(|animation, puppet| animation.play(puppet.unwrap(), "switch", Priority::Normal));
    assert_eq!(played, Ok(true));
    let mut sent_curves = false;
    for _ in 0..60 {
        let sent = conductor.tick(1.0 / 60.0).unwrap();
        sent_curves |= !sent.part_opacities.is_empty();
        follower.receive(&sent).unwrap();
    }
    assert!(sent_curves);
    let puppet = follower.puppet().unwrap();
    let arm_b = puppet.instance().compiled().part_index("ArmB").unwrap();
    assert_eq!(puppet.instance().part_opacity_override(arm_b), (1.0, 1.0));
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

/// ポーズの不透明度 (要素の順) と、比べる状態。
fn pose_shape(puppet: &Puppet) -> Vec<f32> {
    puppet
        .pose()
        .opacities()
        .map(|(_, o)| o)
        .chain(shape(puppet))
        .collect()
}

/// 仮想カメラの受け手 (`virtual_camera.rs` の `Feed::set_scene`) と同じ規則でシーンの通知を受ける: モデルの版が変わったら
/// 読み直し、そうでなければ続ける。読み直し・付け直しの直後だけポーズの要素をフェードせずに表示させる。
fn receive_scene(
    follower: &mut Follower,
    followed: &mut Option<(u32, u32)>,
    scene: &Scene,
    source: &Bundle,
) {
    let reloaded = match *followed {
        Some((model, modules)) if model == scene.model_revision => {
            follower.continue_scene(scene.revision);
            modules != scene.modules_revision
        }
        _ => {
            follower.set_scene(scene.revision, single(source));
            true
        }
    };
    *followed = Some((scene.model_revision, scene.modules_revision));
    follower::show_pose(follower.puppet_mut().unwrap(), &scene.look.pose, reloaded);
}

/// シーンの `pose` を替えた出し直しで、指揮役と受け手のポーズの不透明度と形が各フレームで揃う (T0844)。
/// 同じモデルのまま選び直すとフェードし、モデルを読み直したシーンは選んだ要素からフェードせずに始める。
#[test]
fn pose_switches_of_the_scene_match_the_conductor() {
    let source = bundle(&fixtures::pose_and_effects(), None);
    let model = || {
        assemble(
            NewModel {
                name: "pose.ugoe".into(),
                bundle: source.clone(),
            },
            &[],
        )
        .unwrap()
    };
    let look = |part: &str| Look {
        pose: vec![part.into()],
        ..Look::default()
    };
    let arm = |puppet: &Puppet, id: &str| {
        puppet
            .pose()
            .opacities()
            .find(|(part, _)| *part == id)
            .map(|(_, o)| o)
            .unwrap()
    };
    let mut conductor = Conductor::new();
    let mut follower = Follower::new();
    let mut followed = None;
    let mut faded = false;
    for frame in 0..120 {
        let switch = match frame {
            // モデルを読んだシーン: 選んだ要素 (ArmB) から始める。
            0 => Some((Some(model()), Some(Vec::new()), look("ArmB"))),
            // 同じモデルのまま選び直す: フェードで切り替える。
            30 => Some((None, None, look("ArmA"))),
            // モデルを読み直したシーン: 選んだ要素 (ArmB) からフェードせずに始める。
            80 => Some((Some(model()), Some(Vec::new()), look("ArmB"))),
            _ => None,
        };
        if let Some((model, modules, look)) = switch {
            let scene = conductor.present(model, modules, look).unwrap().clone();
            receive_scene(&mut follower, &mut followed, &scene, &source);
        }
        let sent = conductor.tick(dt_at(frame)).unwrap();
        let sent = Frame::decode(&sent.encode()).unwrap();
        assert_eq!(
            follower.receive(&sent),
            Ok(Received::Applied),
            "frame {frame}"
        );
        let puppet = follower.puppet().unwrap();
        let expected = conductor.perform(|_, conducted| pose_shape(conducted.unwrap()));
        assert_close(&pose_shape(puppet), &expected, &format!("frame {frame}"));
        if frame == 0 || frame == 80 {
            assert_eq!(
                (arm(puppet, "ArmA"), arm(puppet, "ArmB")),
                (0.0, 1.0),
                "frame {frame}"
            );
        }
        let a = arm(puppet, "ArmA");
        faded |= (30..80).contains(&frame) && a > 0.0 && a < 1.0;
    }
    // 同じモデルのままの切り替えは実際にフェードしている (上の検査が空振りしていない)。
    assert!(faded);
}
