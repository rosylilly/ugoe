//! 物理の状態が有限でなくなったときの扱いのテスト (`docs/spec/physics.md#有限でない状態`)。
//!
//! 検証 (`validate_physics`) を通らない設定や、極端な有限値で粒子があふれる設定を、
//! 検証せずに `PhysicsSystem` に渡す (ホストが設定を組み立てる場合)。

use ugoe_model::fixtures::{
    dynamic_hair, dynamic_hair_settings, physics_hair, physics_hair_settings,
};
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::standard::{ANGLE_X, ANGLE_Z};
use ugoe_runtime::{CompiledModel, ModelInstance};

use super::*;

const STEP: f32 = 1.0 / 60.0;

fn set(parameters: &mut ParameterValues, id: &str, value: f32) {
    let index = parameters.index_of(id).unwrap();
    parameters.set(index, value);
}

fn chain_is_finite(physics: &PhysicsSystem) -> bool {
    (0..physics.settings().rigs.len()).all(|r| physics.chain(r).unwrap().is_finite())
}

/// 動的物理の 1 フレーム (`Puppet::update` の物理と評価の部分) を進め、加算量と描画の頂点が
/// 有限であることを確かめる。
fn dynamic_frame(physics: &mut PhysicsSystem, instance: &mut ModelInstance, label: &str) {
    physics.update_instance(instance, STEP);
    instance.update();
    let offsets = instance.dynamic_offsets().unwrap();
    for d in 0..instance.compiled().model().deformers.len() {
        assert!(offsets.angle(d).is_finite(), "{label}: angle of {d}");
        assert!(offsets.origin(d).is_finite(), "{label}: origin of {d}");
    }
    for (m, drawable) in instance.drawables().iter().enumerate() {
        assert!(
            drawable.positions.iter().all(|p| p.is_finite()),
            "{label}: mesh {m}"
        );
    }
    assert!(
        instance
            .parameters()
            .as_slice()
            .iter()
            .all(|v| v.is_finite()),
        "{label}: parameters"
    );
}

/// 有限でない値を作る設定 (検証では拒まれるもの)。
fn broken_settings(base: fn() -> PhysicsSettings) -> Vec<(&'static str, PhysicsSettings)> {
    let mut cases = Vec::new();
    let mut s = base();
    s.gravity = Vec2::new(f32::NAN, 1.0);
    cases.push(("gravity NaN", s));
    let mut s = base();
    s.wind = Vec2::new(f32::INFINITY, 0.0);
    cases.push(("wind infinite", s));
    let mut s = base();
    s.rigs[0].particles[1].radius = f32::INFINITY;
    cases.push(("radius infinite", s));
    let mut s = base();
    s.rigs[0].particles[2].delay = f32::NAN;
    cases.push(("delay NaN", s));
    let mut s = base();
    for output in &mut s.rigs[0].outputs {
        output.scale = f32::INFINITY;
    }
    cases.push(("scale infinite", s));
    let mut s = base();
    // 有限だが、出力の値が単精度であふれる長さ。
    for p in &mut s.rigs[0].particles {
        p.radius = f32::MAX;
    }
    cases.push(("radius f32::MAX", s));
    cases
}

/// パラメータへの出力は、有限でない値を書かない。
#[test]
fn broken_settings_keep_parameters_finite() {
    for (label, settings) in broken_settings(physics_hair_settings) {
        let mut physics = PhysicsSystem::new(settings);
        let mut parameters = ParameterValues::from_model(&physics_hair());
        for k in 0..120 {
            set(&mut parameters, ANGLE_X, 30.0 * (k as f32 * 0.1).sin());
            set(&mut parameters, ANGLE_Z, 20.0 * (k as f32 * 0.07).cos());
            physics.update(&mut parameters, STEP);
            assert!(
                parameters.as_slice().iter().all(|v| v.is_finite()),
                "{label}: frame {k}"
            );
        }
    }
}

/// 動的物理の加算量 (角度・原点・頂点) に有限でない値を足さず、描画の頂点が有限のまま。
#[test]
fn broken_settings_keep_dynamic_offsets_finite() {
    for (label, settings) in broken_settings(dynamic_hair_settings) {
        let mut physics = PhysicsSystem::new(settings);
        let mut instance = ModelInstance::new(CompiledModel::compile(dynamic_hair()).unwrap());
        for k in 0..120 {
            let index = instance.parameters().index_of(ANGLE_X).unwrap();
            instance
                .parameters_mut()
                .set(index, 30.0 * (k as f32 * 0.1).sin());
            dynamic_frame(&mut physics, &mut instance, label);
        }
    }
}

/// 根元があふれて粒子が有限でなくなっても、そのリグを静止姿勢に置き直し、入力が戻れば有限の鎖に戻る
/// (`reset` を呼ばなくても NaN のまま止まらない)。
#[test]
fn chain_recovers_after_overflowing_input() {
    let mut settings = dynamic_hair_settings();
    // スカートの入力の重みを有限の最大にする。AngleX が最大なら根元が無限大になり、0 なら 0。
    settings.rigs[1].inputs[0].weight = f32::MAX;
    let mut physics = PhysicsSystem::new(settings);
    let mut instance = ModelInstance::new(CompiledModel::compile(dynamic_hair()).unwrap());
    for _ in 0..10 {
        dynamic_frame(&mut physics, &mut instance, "rest");
    }
    let cloth = instance.compiled().deformer_index("Cloth").unwrap();
    let rest = instance.warp_world_points(cloth).unwrap().to_vec();

    let index = instance.parameters().index_of(ANGLE_X).unwrap();
    instance.parameters_mut().set(index, 30.0);
    for _ in 0..10 {
        dynamic_frame(&mut physics, &mut instance, "overflow");
    }
    // 根元が無限大の間、スカートの鎖は有限でない (静止姿勢も無限大) が、出力は捨てている。
    assert!(!physics.chain(1).unwrap().is_finite());

    instance.parameters_mut().set(index, 0.0);
    for _ in 0..3 {
        dynamic_frame(&mut physics, &mut instance, "back");
    }
    assert!(chain_is_finite(&physics));
    // 戻った直後は静止姿勢に置き直しているので、スカートの頂点は元の位置にある。
    let points = instance.warp_world_points(cloth).unwrap();
    assert!(
        points[2].abs_diff_eq(rest[2], 0.05),
        "{} {}",
        points[2],
        rest[2]
    );
}

/// 有限の設定では、置き直しも出力を捨てることも起きない (従来の結果のまま。`golden_tests` でも確かめる)。
#[test]
fn finite_settings_keep_chains_finite() {
    let mut physics = PhysicsSystem::new(physics_hair_settings());
    let mut parameters = ParameterValues::from_model(&physics_hair());
    for k in 0..120 {
        set(&mut parameters, ANGLE_X, 30.0 * (k as f32 * 0.3).sin());
        physics.update(&mut parameters, STEP);
        assert!(chain_is_finite(&physics));
    }
}
