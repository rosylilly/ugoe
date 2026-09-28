//! 物理の出力の合成の方法 (`add`) のテスト (`docs/spec/physics.md#合成の方法`)。
//! 入力は `ugoe_model::fixtures::physics_additive` とその物理設定。
//!
//! `Puppet` はフレームごとにパラメータを基準値に戻してから物理を進める
//! (`docs/spec/motion.md#フレーム内の適用順`)。ここでは毎フレーム、基準値 (トラッキングの値) を
//! 書き直してから `update` を呼んで同じ順序を再現する。

use ugoe_model::fixtures::{
    RELAY_BOUNCE, RELAY_FOLLOW, physics_additive, physics_additive_settings, physics_relay,
    physics_relay_settings,
};
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::standard::{ANGLE_X, BODY_ANGLE_X, BODY_ANGLE_Z};

use super::*;

const STEP: f32 = 1.0 / 60.0;

/// トラッキングで体の回転がないときに、頭の回転から体の回転を作る係数
/// (`ugoe_motion::tracking::BODY_FROM_HEAD`)。
const BODY_FROM_HEAD: f32 = 0.3;

/// 頭を振る先の `AngleX`。体の回転 (6) に揺れを足しても `BodyAngleX` の値域 (-10..10) に収まる。
const YAW: f32 = 20.0;

fn get(params: &ParameterValues, id: &str) -> f32 {
    params.get(params.index_of(id).unwrap())
}

fn set(params: &mut ParameterValues, id: &str, value: f32) {
    let index = params.index_of(id).unwrap();
    params.set(index, value);
}

fn with_blend(mut settings: PhysicsSettings, blend: PhysicsBlend) -> PhysicsSettings {
    for output in settings.rigs.iter_mut().flat_map(|r| &mut r.outputs) {
        output.blend = blend;
    }
    settings
}

/// 頭を 10 フレーム正面に向けてから `AngleX` を [`YAW`] まで振り、`frames` フレーム分の `BodyAngleX` を返す。
/// 毎フレーム、トラッキングと同じく `AngleX` と `BodyAngleX` (= `AngleX` × 0.3) を書いてから進める。
fn head_turn(settings: PhysicsSettings, frames: usize) -> Vec<f32> {
    let mut physics = PhysicsSystem::new(settings);
    let mut params = ParameterValues::from_model(&physics_additive());
    (0..10 + frames)
        .map(|frame| {
            let yaw = if frame < 10 { 0.0 } else { YAW };
            set(&mut params, ANGLE_X, yaw);
            set(&mut params, BODY_ANGLE_X, yaw * BODY_FROM_HEAD);
            physics.update(&mut params, STEP);
            get(&params, BODY_ANGLE_X)
        })
        .skip(10)
        .collect()
}

/// 静止しているときは足す量が 0 で、値は基準値 (物理を進める前の値) のまま。
/// 頭を回したまま読み込んでも (最初の `update` で静止姿勢に置く) 同じ。
#[test]
fn add_contributes_nothing_at_rest() {
    for yaw in [0.0, 30.0, -12.0] {
        let mut physics = PhysicsSystem::new(physics_additive_settings());
        let mut params = ParameterValues::from_model(&physics_additive());
        for _ in 0..30 {
            set(&mut params, ANGLE_X, yaw);
            set(&mut params, BODY_ANGLE_X, 3.5);
            physics.update(&mut params, STEP);
            assert!(
                (get(&params, BODY_ANGLE_X) - 3.5).abs() < 1e-5,
                "yaw {yaw}: {}",
                get(&params, BODY_ANGLE_X)
            );
        }
    }
}

/// 静止時の値は、その時の入力での静止姿勢の出力。重力が斜めで風があり、角度を出力しても、
/// 静止していれば足す量は 0 (`overwrite` なら静止時の値そのものが書き込まれる)。
#[test]
fn rest_value_follows_gravity_and_wind() {
    let mut settings = physics_additive_settings();
    settings.gravity = Vec2::new(0.3, 1.0);
    settings.wind = Vec2::new(0.5, 0.0);
    settings.rigs[0].outputs[0].source = PhysicsSource::Angle;
    settings.rigs[0].outputs[0].scale = 0.1;
    let run = |blend| {
        let mut physics = PhysicsSystem::new(with_blend(settings.clone(), blend));
        let mut params = ParameterValues::from_model(&physics_additive());
        for _ in 0..3 {
            set(&mut params, ANGLE_X, 20.0);
            set(&mut params, BODY_ANGLE_X, -2.0);
            physics.update(&mut params, STEP);
        }
        get(&params, BODY_ANGLE_X)
    };
    assert!((run(PhysicsBlend::Add) + 2.0).abs() < 1e-5);
    // 風で傾いて垂れるので、上書きなら 0 でない角度になる (足す量を引かないと値がずれる)。
    assert!(run(PhysicsBlend::Overwrite).abs() > 0.1);
}

/// パラメータを複製せずに、トラッキングの `AngleX` から同じ `BodyAngleX` に
/// 「トラッキング × 係数 + 物理の遅れ」を出す。
#[test]
fn tracking_value_plus_physics_lag_on_one_parameter() {
    let target = YAW * BODY_FROM_HEAD;
    let trace = head_turn(physics_additive_settings(), 300);

    // 頭が動いた直後は、体が遅れる (トラッキングの値より小さい)。
    let min = trace[..30].iter().copied().fold(f32::INFINITY, f32::min);
    assert!(min < target - 1.0, "lag: min {min}");
    // その後、行き過ぎてから戻る。
    let max = trace[..120]
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(max > target + 0.1, "overshoot: max {max}");
    // 揺れが収まるとトラッキングの値に戻る (フレームごとに累積しない)。
    for (frame, v) in trace.iter().enumerate().skip(240) {
        assert!((v - target).abs() < 0.05, "frame {frame}: {v}");
    }

    // 上書きでは、トラッキングの値が物理の値 (静止時は x = 0) に置き換わってしまう。
    let overwrite = head_turn(
        with_blend(physics_additive_settings(), PhysicsBlend::Overwrite),
        300,
    );
    assert!(overwrite[299].abs() < 0.05, "{}", overwrite[299]);
    // 加算の揺れは、上書きの値を基準値の分だけずらしたもの (重み 100、クランプしない範囲)。
    for (add, over) in trace.iter().zip(&overwrite) {
        assert!((add - target - over).abs() < 1e-4, "{add} vs {over}");
    }
}

/// 重みは、静止時の値からのずれに掛かる。
#[test]
fn weight_scales_the_deviation() {
    let target = YAW * BODY_FROM_HEAD;
    let mut half = physics_additive_settings();
    half.rigs[0].outputs[0].weight = 50.0;
    let full = head_turn(physics_additive_settings(), 60);
    let half = head_turn(half, 60);
    for (f, h) in full.iter().zip(&half) {
        assert!(
            ((h - target) - (f - target) * 0.5).abs() < 1e-4,
            "{f} vs {h}"
        );
    }
}

/// 足した結果はパラメータの値域にクランプする。
#[test]
fn add_is_clamped_to_the_range() {
    let mut settings = physics_additive_settings();
    settings.rigs[0].outputs[0].scale = 10.0;
    let trace = head_turn(settings, 120);
    assert!(trace.iter().all(|v| (-10.0..=10.0).contains(v)));
    assert!(trace.contains(&-10.0) && trace.contains(&10.0));
}

/// 未知の合成の方法は `overwrite` として扱う。
#[test]
fn unknown_blend_is_overwrite() {
    let unknown = head_turn(
        with_blend(physics_additive_settings(), PhysicsBlend::Unknown),
        60,
    );
    let overwrite = head_turn(
        with_blend(physics_additive_settings(), PhysicsBlend::Overwrite),
        60,
    );
    assert_eq!(unknown, overwrite);
}

/// 中継の入力は、加算した後の値 (物理を進める前の値 + ずれ) を読む。
/// 前のリグの出力先の基準値だけを動かしても、後ろのリグが動く。
#[test]
fn relay_reads_the_added_value() {
    let run = |blend| {
        let mut settings = physics_relay_settings();
        settings.rigs[0].outputs[0].blend = blend;
        let mut physics = PhysicsSystem::new(settings);
        let mut params = ParameterValues::from_model(&physics_relay());
        (0..40)
            .map(|frame| {
                // `BodyAngleX` は動かさず、中継パラメータの基準値だけを途中で 5 にする。
                set(
                    &mut params,
                    RELAY_FOLLOW,
                    if frame < 10 { 0.0 } else { 5.0 },
                );
                physics.update(&mut params, STEP);
                (get(&params, RELAY_FOLLOW), get(&params, RELAY_BOUNCE))
            })
            .collect::<Vec<_>>()
    };
    let add = run(PhysicsBlend::Add);
    // 加算: 基準値の 5 が見え、後ろのリグ (弾み) が揺れる。
    assert!((add[39].0 - 5.0).abs() < 1e-3, "{:?}", add[39]);
    assert!(add[10..].iter().any(|(_, bounce)| bounce.abs() > 0.1));
    // 上書き: 物理の値に置き換わるので、基準値は見えず、後ろのリグは動かない。
    let overwrite = run(PhysicsBlend::Overwrite);
    assert!(
        overwrite
            .iter()
            .all(|(follow, bounce)| *follow == 0.0 && *bounce == 0.0)
    );
}

/// 基準値が 0 のままなら、中継の連鎖を含めて加算と上書きは同じ結果になる
/// (静止時の x は 0 なので、`0 + (x - 0)` と `x` が等しい)。
#[test]
fn add_on_zero_base_matches_overwrite_through_the_relay() {
    let run = |blend| {
        let mut settings = physics_relay_settings();
        settings.rigs[0].outputs[0].blend = blend;
        let mut physics = PhysicsSystem::new(settings);
        let mut params = ParameterValues::from_model(&physics_relay());
        (0..60)
            .map(|frame| {
                set(
                    &mut params,
                    BODY_ANGLE_X,
                    if frame < 10 { 0.0 } else { 10.0 },
                );
                set(&mut params, RELAY_FOLLOW, 0.0);
                set(&mut params, RELAY_BOUNCE, 0.0);
                set(&mut params, BODY_ANGLE_Z, 0.0);
                physics.update(&mut params, STEP);
                [RELAY_FOLLOW, RELAY_BOUNCE, BODY_ANGLE_Z].map(|id| get(&params, id))
            })
            .collect::<Vec<_>>()
    };
    let add = run(PhysicsBlend::Add);
    let overwrite = run(PhysicsBlend::Overwrite);
    for (a, o) in add.iter().zip(&overwrite) {
        for (a, o) in a.iter().zip(o) {
            assert!((a - o).abs() < 1e-4, "{a} vs {o}");
        }
    }
    assert!(add.iter().any(|v| v[2].abs() > 0.1), "the chain must move");
}
