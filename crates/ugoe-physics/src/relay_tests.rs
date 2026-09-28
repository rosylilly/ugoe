//! リグの順序と中継パラメータのテスト (`docs/spec/physics.md#リグの順序と中継`)。
//! 入力は `ugoe_model::fixtures::physics_relay` とその物理設定。

use ugoe_model::Parameter;
use ugoe_model::fixtures::{
    RELAY_BOUNCE, RELAY_FOLLOW, dynamic_hair, dynamic_hair_settings, physics_hair_settings,
    physics_relay, physics_relay_settings,
};
use ugoe_model::physics::{PhysicsInput, PhysicsOutput, PhysicsSettings};
use ugoe_model::standard::{ANGLE_X, BODY_ANGLE_X, BODY_ANGLE_Z};
use ugoe_runtime::{CompiledModel, ModelInstance};

use super::*;

const STEP: f32 = 1.0 / 60.0;

fn get(params: &ParameterValues, id: &str) -> f32 {
    params.get(params.index_of(id).unwrap())
}

fn set(params: &mut ParameterValues, id: &str, value: f32) {
    let index = params.index_of(id).unwrap();
    params.set(index, value);
}

/// 連鎖の出力 (追従・弾み・角度) の順。
const CHAIN: [&str; 3] = [RELAY_FOLLOW, RELAY_BOUNCE, BODY_ANGLE_Z];

/// 静止させてから `BodyAngleX` を 10 にし、各出力が初めて 0 でなくなったフレームを返す。
fn first_moved_frames(settings: PhysicsSettings) -> [usize; 3] {
    let mut physics = PhysicsSystem::new(settings);
    let mut params = ParameterValues::from_model(&physics_relay());
    for _ in 0..10 {
        physics.update(&mut params, STEP);
    }
    for id in CHAIN {
        assert_eq!(get(&params, id), 0.0, "{id} must rest before the move");
    }
    set(&mut params, BODY_ANGLE_X, 10.0);
    let mut first = [None; 3];
    for frame in 0..20 {
        physics.update(&mut params, STEP);
        for (slot, id) in first.iter_mut().zip(CHAIN) {
            if slot.is_none() && get(&params, id).abs() > 1e-6 {
                *slot = Some(frame);
            }
        }
    }
    first.map(|f| f.expect("every output of the chain must move"))
}

/// 前のリグの出力は、後のリグが同じステップで読む。連鎖しても遅れは増えない。
#[test]
fn later_rig_reads_earlier_output_in_the_same_step() {
    let settings = physics_relay_settings();
    let physics = PhysicsSystem::new(settings.clone());
    assert!(physics.has_relay());
    let [follow, bounce, angle] = first_moved_frames(settings);
    assert_eq!((bounce, angle), (follow, follow));
}

/// 順序を逆にすると、中継の入力はフレームの始めの値 (前のフレームに書いた、補間した出力) を読むので、
/// 連鎖の段ごとに遅れる。1 フレーム 1 ステップでは、読むのが次のフレームになる分と、
/// 出力の補間の遅れ (1 ステップ) の分で、段ごとに 2 フレーム遅れる。
#[test]
fn reversed_order_delays_each_link() {
    let mut settings = physics_relay_settings();
    settings.rigs.reverse();
    assert!(!PhysicsSystem::new(settings.clone()).has_relay());
    let [follow, bounce, angle] = first_moved_frames(settings);
    assert_eq!((bounce, angle), (follow + 2, follow + 4));
}

/// 順序を入れ替えると、動いている間の値も変わる。
#[test]
fn order_changes_the_result() {
    let run = |settings: PhysicsSettings| {
        let mut physics = PhysicsSystem::new(settings);
        let mut params = ParameterValues::from_model(&physics_relay());
        let mut trace = Vec::new();
        for k in 0..120 {
            set(&mut params, BODY_ANGLE_X, (k as f32 * 0.05).sin() * 10.0);
            physics.update(&mut params, STEP);
            trace.push(get(&params, BODY_ANGLE_Z));
        }
        trace
    };
    let ordered = run(physics_relay_settings());
    let mut settings = physics_relay_settings();
    settings.rigs.swap(1, 2);
    let swapped = run(settings);
    assert!(ordered.iter().any(|v| v.abs() > 0.1), "the chain must move");
    let diff = ordered
        .iter()
        .zip(&swapped)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max);
    assert!(diff > 0.01, "max difference {diff}");
}

/// 最初の `update` の静止姿勢も、前のリグの静止姿勢の出力から求める。
#[test]
fn settle_uses_earlier_outputs() {
    // 入力を角度にすると、静止姿勢は入力の角度だけ傾き、出力が 0 でなくなる。
    let settings = |reversed: bool| {
        let mut settings = physics_relay_settings();
        settings.rigs[0].inputs[0].source = PhysicsSource::Angle;
        settings.rigs[1].inputs[0].source = PhysicsSource::Angle;
        if reversed {
            settings.rigs.reverse();
        }
        settings
    };
    let settle = |settings| {
        let mut physics = PhysicsSystem::new(settings);
        let mut params = ParameterValues::from_model(&physics_relay());
        set(&mut params, BODY_ANGLE_X, 10.0);
        physics.update(&mut params, 0.0);
        CHAIN.map(|id| get(&params, id))
    };
    let [follow, bounce, angle] = settle(settings(false));
    assert!(follow.abs() > 1.0, "follow {follow}");
    // 静止姿勢の粒子 1 の角度は入力の角度と同じ (値域と正規化の範囲が同じなので、値がそのまま角度になる)。
    assert!((bounce - 0.2 * follow).abs() < 1e-4, "{bounce} {follow}");
    assert!((angle - bounce).abs() < 1e-4, "{angle} {bounce}");
    // 逆の順では、後ろのリグの出力はまだ読めない。
    let [follow, bounce, angle] = settle(settings(true));
    assert!(follow.abs() > 1.0);
    assert_eq!((bounce, angle), (0.0, 0.0));
}

/// `docs/spec/physics.md#決定性`: 中継があっても、同じ入力列から同じ出力 (ビット単位で一致)。
#[test]
fn relay_is_deterministic() {
    let run = || {
        let mut physics = PhysicsSystem::new(physics_relay_settings());
        let mut params = ParameterValues::from_model(&physics_relay());
        let mut trace = Vec::new();
        for k in 0..300 {
            let t = k as f32 * 0.021;
            set(&mut params, BODY_ANGLE_X, (t * 2.3).sin() * 10.0);
            let dt = [1.0 / 60.0, 1.0 / 144.0, 1.0 / 30.0, 0.1][k % 4];
            physics.update(&mut params, dt);
            trace.push(params.as_slice().to_vec());
        }
        trace
    };
    let a = run();
    assert_eq!(a, run());
    assert!(a.iter().any(|v| v[3].abs() > 0.1));
}

/// 中継のない設定 (既存のフィクスチャ) は、順序付きの評価にしても中継を持たない。
#[test]
fn fixtures_without_relay() {
    assert!(!PhysicsSystem::new(physics_hair_settings()).has_relay());
    assert!(!PhysicsSystem::new(dynamic_hair_settings()).has_relay());
}

/// 同じパラメータを前のリグと自分が出力するときも、中継の入力は前のリグの出力だけを読む。
#[test]
fn relay_input_ignores_own_output() {
    let mut settings = physics_relay_settings();
    // 弾みのリグも `PhysBodyFollow` に書く (重み 0 なので値は変えない)。
    let output = PhysicsOutput {
        weight: 0.0,
        ..settings.rigs[0].outputs[0].clone()
    };
    settings.rigs[1].outputs.push(output);
    let plain = first_moved_frames(physics_relay_settings());
    assert_eq!(first_moved_frames(settings), plain);
}

/// 動的物理と組み合わせる: パラメータを出力するリグの後の、デフォーマに出力するリグが中継を読む。
#[test]
fn relay_drives_dynamic_rig() {
    let sway = "HairSway";
    let settings = |ordered: bool| {
        let mut settings = dynamic_hair_settings();
        let mut rig = physics_relay_settings().rigs.remove(0);
        rig.id = "Sway".into();
        rig.inputs[0].parameter = ANGLE_X.into();
        rig.outputs[0] = PhysicsOutput {
            target: PhysicsTarget::Parameter { id: sway.into() },
            ..rig.outputs[0].clone()
        };
        settings.rigs[0].inputs.push(PhysicsInput {
            parameter: sway.into(),
            source: PhysicsSource::X,
            weight: 100.0,
            reflect: false,
        });
        if ordered {
            settings.rigs.insert(0, rig);
        } else {
            settings.rigs.push(rig);
        }
        settings
    };
    let run = |settings: PhysicsSettings| {
        let mut model = dynamic_hair();
        model.parameters.push(Parameter {
            internal: true,
            ..model.parameters[0].clone()
        });
        model.parameters.last_mut().unwrap().id = sway.into();
        let mut instance = ModelInstance::new(CompiledModel::compile(model).unwrap());
        let mut physics = PhysicsSystem::new(settings);
        assert!(physics.is_dynamic());
        let hair = instance.compiled().deformer_index("Hair1").unwrap();
        let mut trace = Vec::new();
        for k in 0..120 {
            let index = instance.parameters().index_of(ANGLE_X).unwrap();
            instance
                .parameters_mut()
                .set(index, (k as f32 * 0.07).sin() * 30.0);
            physics.update_instance(&mut instance, STEP);
            instance.update();
            trace.push(instance.dynamic_offsets().unwrap().angle(hair));
        }
        (physics.has_relay(), trace)
    };
    let (relay, ordered) = run(settings(true));
    assert!(relay);
    assert_eq!(run(settings(true)).1, ordered, "deterministic");
    let (relay, reversed) = run(settings(false));
    assert!(!relay);
    assert!(ordered.iter().any(|v| v.abs() > 0.5), "the hair must swing");
    let diff = ordered
        .iter()
        .zip(&reversed)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max);
    assert!(diff > 1e-3, "max difference {diff}");
}
