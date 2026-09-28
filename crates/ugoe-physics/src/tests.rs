//! `PhysicsSystem` のテスト。入力は `ugoe_model::fixtures::physics_hair` とその物理設定。

use ugoe_model::fixtures::{physics_hair, physics_hair_settings};
use ugoe_model::physics::PhysicsTarget;
use ugoe_model::standard::{ANGLE_X, ANGLE_Z, HAIR_BACK, HAIR_FRONT};

use super::*;

const STEP: f32 = 1.0 / 60.0;

fn setup() -> (PhysicsSystem, ParameterValues) {
    (
        PhysicsSystem::new(physics_hair_settings()),
        ParameterValues::from_model(&physics_hair()),
    )
}

fn set(params: &mut ParameterValues, id: &str, value: f32) {
    let index = params.index_of(id).unwrap();
    params.set(index, value);
}

fn get(params: &ParameterValues, id: &str) -> f32 {
    params.get(params.index_of(id).unwrap())
}

fn assert_close(a: f32, b: f32, eps: f32) {
    assert!((a - b).abs() <= eps, "{a} != {b} (eps {eps})");
}

fn positions(physics: &PhysicsSystem) -> Vec<Vec2> {
    (0..physics.settings().rigs.len())
        .flat_map(|r| {
            physics
                .chain(r)
                .unwrap()
                .particles()
                .iter()
                .map(|p| p.position)
        })
        .collect()
}

#[test]
fn rest_input_keeps_outputs_at_default() {
    let (mut physics, mut params) = setup();
    for _ in 0..120 {
        physics.update(&mut params, STEP);
    }
    assert_close(get(&params, HAIR_FRONT), 0.0, 1e-5);
    assert_close(get(&params, HAIR_BACK), 0.0, 1e-5);
}

/// `docs/spec/physics.md#決定性`: 同じ入力列から同じ出力 (ビット単位で一致)。
#[test]
fn same_input_sequence_gives_same_output() {
    let run = || {
        let (mut physics, mut params) = setup();
        let mut trace = Vec::new();
        for k in 0..300 {
            let t = k as f32 * 0.021;
            set(&mut params, ANGLE_X, (t * 2.3).sin() * 30.0);
            set(&mut params, ANGLE_Z, (t * 1.7).cos() * 20.0);
            // フレームの長さもばらつかせる。
            let dt = [1.0 / 60.0, 1.0 / 144.0, 1.0 / 30.0, 0.1][k % 4];
            physics.update(&mut params, dt);
            trace.push(params.as_slice().to_vec());
        }
        trace
    };
    let a = run();
    assert_eq!(a, run());
    // 揺れていることも確かめる (全部 0 なら上の比較に意味がない)。
    assert!(a.iter().any(|v| v[2].abs() > 0.05));
}

/// `docs/spec/physics.md#入力`: default の両側で別々に線形写像する。
#[test]
fn normalize_maps_each_side_of_default_separately() {
    let mut model = physics_hair();
    let angle_x = model
        .parameters
        .iter_mut()
        .find(|p| p.id.as_str() == ANGLE_X)
        .unwrap();
    angle_x.min = -10.0;
    angle_x.max = 30.0;
    let mut params = ParameterValues::from_model(&model);
    let index = params.index_of(ANGLE_X).unwrap();
    let range = NormalizationRange {
        min: -4.0,
        default: 1.0,
        max: 11.0,
    };
    for (value, expected) in [
        (0.0, 1.0),
        (15.0, 6.0),
        (30.0, 11.0),
        (-5.0, -1.5),
        (-10.0, -4.0),
    ] {
        params.set(index, value);
        assert_close(normalize(&params, index, range), expected, 1e-5);
    }
}

/// 最初の `update` で静止姿勢に置くので、頭を傾けたまま読み込んでも揺れ始めない。
/// 静止姿勢では、根元の角度だけ重力が回り、粒子 1 の角度 = 入力の角度になる。
#[test]
fn first_update_settles_at_steady_state() {
    let (mut physics, mut params) = setup();
    set(&mut params, ANGLE_Z, 30.0);
    physics.update(&mut params, STEP);
    // AngleZ = 30 → 正規化で 10 度。重み 60% で 6 度、scale 0.1 で 0.6。
    assert_close(get(&params, HAIR_FRONT), 0.6, 1e-4);
    assert_close(get(&params, HAIR_BACK), 0.4, 1e-4);
    for _ in 0..60 {
        physics.update(&mut params, STEP);
        assert_close(get(&params, HAIR_FRONT), 0.6, 1e-4);
    }
}

#[test]
fn moving_head_swings_and_settles() {
    let (mut physics, mut params) = setup();
    physics.update(&mut params, STEP);
    set(&mut params, ANGLE_X, 30.0);
    let mut front_peak = 0.0f32;
    let mut back_first = 0.0;
    for k in 0..600 {
        physics.update(&mut params, STEP);
        let front = get(&params, HAIR_FRONT);
        if front.abs() > front_peak.abs() {
            front_peak = front;
        }
        if k == 3 {
            back_first = get(&params, HAIR_BACK);
        }
    }
    // 根元が右に動くと髪は遅れて左に傾く (時計回り、正)。
    assert!(front_peak > 0.1, "peak {front_peak}");
    // HairBack は x の入力を反転しているので逆向きに揺れる。
    assert!(back_first < 0.0, "back {back_first}");
    // x の入力は静止姿勢を変えないので、揺れが収まると 0 に戻る。
    assert_close(get(&params, HAIR_FRONT), 0.0, 1e-3);
    assert_close(get(&params, HAIR_BACK), 0.0, 1e-3);
}

/// `docs/spec/physics.md#時間の進め方`: 固定ステップなので、フレームの分け方で結果が変わらない。
#[test]
fn frame_splitting_does_not_change_steps() {
    let run = |dts: &[f32]| {
        let (mut physics, mut params) = setup();
        physics.update(&mut params, 0.0);
        set(&mut params, ANGLE_Z, -30.0);
        // 入力の変化はステップの間で補間されるので、変化を 1 ステップで吸収してから比べる。
        physics.update(&mut params, STEP);
        for _ in 0..20 {
            for &dt in dts {
                physics.update(&mut params, dt);
            }
        }
        positions(&physics)
    };
    // 入力が一定なら、1 回で 2 ステップ進めても 1 ステップずつ 2 回でも同じ。
    let a = run(&[STEP * 2.0]);
    let b = run(&[STEP, STEP]);
    let c = run(&[STEP * 0.5, STEP * 1.5]);
    for ((a, b), c) in a.iter().zip(&b).zip(&c) {
        assert!((*a - *b).length() < 1e-4);
        assert!((*a - *c).length() < 1e-4);
    }
}

/// `docs/spec/physics.md#時間の進め方`: 1 回の update のステップ数は上限で打ち切り、残りの時間は捨てる。
#[test]
fn step_count_is_capped() {
    let run = |dt: f32| {
        let (mut physics, mut params) = setup();
        physics.update(&mut params, 0.0);
        set(&mut params, ANGLE_X, 30.0);
        physics.update(&mut params, dt);
        (positions(&physics), physics.accumulator)
    };
    let (capped, accumulator) = run(10.0);
    let (exact, _) = run(STEP * (MAX_STEPS_PER_UPDATE as f32 + 0.5));
    assert_eq!(capped, exact);
    assert!(accumulator < STEP);
}

/// `docs/spec/physics.md#時間の進め方`: 出力は直前 2 ステップの結果を余りの時間で補間する。
#[test]
fn output_is_interpolated_between_steps() {
    let (mut physics, mut params) = setup();
    physics.update(&mut params, 0.0);
    // 出力が値域でクランプされない程度の小さな動き。
    set(&mut params, ANGLE_X, 1.0);
    // 1 ステップ進めた直後は余りが 0 なので、1 つ前 (静止姿勢) の出力になる。
    physics.update(&mut params, STEP);
    assert_close(get(&params, HAIR_FRONT), 0.0, 1e-6);
    // 半ステップ: 静止姿勢と 1 ステップ目の中間。
    physics.update(&mut params, STEP * 0.5);
    let half = get(&params, HAIR_FRONT);
    // もう半ステップで 2 ステップ目を実行し、余り 0 で 1 ステップ目の出力になる。
    physics.update(&mut params, STEP * 0.5);
    let first = get(&params, HAIR_FRONT);
    assert!(first.abs() > 1e-3);
    assert_close(half, first * 0.5, 1e-6);
}

#[test]
fn invalid_dt_is_ignored() {
    let (mut physics, mut params) = setup();
    physics.update(&mut params, 0.0);
    set(&mut params, ANGLE_X, 30.0);
    let before = positions(&physics);
    for dt in [f32::NAN, -1.0, f32::INFINITY, 0.0] {
        physics.update(&mut params, dt);
    }
    assert_eq!(positions(&physics), before);
}

#[test]
fn reset_settles_again_on_next_update() {
    let (mut physics, mut params) = setup();
    physics.update(&mut params, 0.0);
    set(&mut params, ANGLE_X, 30.0);
    for _ in 0..5 {
        physics.update(&mut params, STEP);
    }
    assert!(get(&params, HAIR_FRONT).abs() > 1e-3);
    physics.reset();
    physics.update(&mut params, STEP);
    assert_close(get(&params, HAIR_FRONT), 0.0, 1e-5);
}

#[test]
fn missing_parameters_and_bad_particles_are_ignored() {
    let mut settings = physics_hair_settings();
    settings.rigs[0].inputs[0].parameter = "Missing".into();
    settings.rigs[0].outputs[0].particle = 99;
    settings.rigs[1].outputs[0].target = PhysicsTarget::Parameter {
        id: "Missing".into(),
    };
    settings.fps = 0.0;
    let mut physics = PhysicsSystem::new(settings);
    let mut params = ParameterValues::from_model(&physics_hair());
    assert_close(physics.step_duration(), 1.0 / DEFAULT_FPS, 1e-9);
    set(&mut params, ANGLE_X, 30.0);
    for _ in 0..10 {
        physics.update(&mut params, STEP);
    }
    assert_eq!(get(&params, HAIR_FRONT), 0.0);
    assert_eq!(get(&params, HAIR_BACK), 0.0);
}

/// 出力の重みは、現在値とのブレンドの割合。
#[test]
fn output_weight_blends_with_current_value() {
    let mut settings = physics_hair_settings();
    settings.rigs[0].outputs[0].weight = 50.0;
    let mut physics = PhysicsSystem::new(settings);
    let mut params = ParameterValues::from_model(&physics_hair());
    set(&mut params, ANGLE_Z, 30.0);
    set(&mut params, HAIR_FRONT, 1.0);
    physics.update(&mut params, 0.0);
    // (1.0 + 0.6) / 2
    assert_close(get(&params, HAIR_FRONT), 0.8, 1e-4);
}
