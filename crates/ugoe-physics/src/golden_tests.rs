//! 結果がビット単位で変わらないことの確かめ (T0319)。
//!
//! 毎フレームの確保と名前の探索を減らす前の実装で、決まった入力列の結果 (パラメータの値・粒子・
//! 描画の頂点) のハッシュを記録した。実装を変えてもこの値が変わらなければ、結果はビット単位で同じ。
//! 結果を意図して変える (仕様の変更) ときは、理由を書いてハッシュを更新する。

use ugoe_core::math;
use ugoe_model::Model;
use ugoe_model::fixtures::{
    dynamic_hair, dynamic_hair_settings, physics_hair, physics_hair_settings, physics_relay,
    physics_relay_settings,
};
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::standard::{ANGLE_X, ANGLE_Y, ANGLE_Z, BODY_ANGLE_X};
use ugoe_runtime::{CompiledModel, ModelInstance};

use super::*;

const STEP: f32 = 1.0 / 60.0;

/// FNV-1a (64 ビット) で `f32` のビット列を畳む。
struct Hasher(u64);

impl Hasher {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn f32(&mut self, v: f32) {
        for b in v.to_bits().to_le_bytes() {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn vec2(&mut self, v: Vec2) {
        self.f32(v.x);
        self.f32(v.y);
    }

    fn physics(&mut self, physics: &PhysicsSystem) {
        for rig in 0..physics.settings().rigs.len() {
            for p in physics.chain(rig).unwrap().particles() {
                self.vec2(p.position);
                self.vec2(p.velocity);
            }
        }
    }
}

/// フレーム `k` の入力の値と `dt` (表示のフレームレートの揺れを含む)。
fn input(k: usize) -> (f32, f32, f32) {
    let x = 30.0 * math::sin(k as f32 * 0.07);
    let z = 20.0 * math::cos(k as f32 * 0.03);
    let dt = match k % 4 {
        0 => STEP * 1.7,
        1 => STEP * 0.6,
        2 => STEP * 3.1,
        _ => STEP,
    };
    (x, z, dt)
}

fn set(params: &mut ParameterValues, id: &str, value: f32) {
    if let Some(index) = params.index_of(id) {
        params.set(index, value);
    }
}

/// パラメータの並びを逆にしたモデル (渡す `ParameterValues` の並びが変わる場合)。
fn reversed(mut model: Model) -> Model {
    model.parameters.reverse();
    model
}

/// `update` で 240 フレーム進めたハッシュ。`layouts` の並びの `ParameterValues` を 30 フレームごとに切り替える
/// (値は前の並びから引き継ぐ)。
fn run_update(settings: PhysicsSettings, layouts: &[Model]) -> u64 {
    let mut physics = PhysicsSystem::new(settings);
    let mut params = ParameterValues::from_model(&layouts[0]);
    let mut hash = Hasher::new();
    for k in 0..240 {
        if k % 30 == 0 && k > 0 {
            let mut next = ParameterValues::from_model(&layouts[(k / 30) % layouts.len()]);
            next.carry_over(&params);
            params = next;
        }
        let (x, z, dt) = input(k);
        set(&mut params, ANGLE_X, x);
        set(&mut params, ANGLE_Z, z);
        set(&mut params, ANGLE_Y, -z);
        set(&mut params, BODY_ANGLE_X, x / 3.0);
        physics.update(&mut params, dt);
        // 並びによらず、ID の順で畳む。
        let mut values: Vec<(&str, f32)> = layouts[0]
            .parameters
            .iter()
            .filter_map(|p| Some((p.id.as_str(), params.get(params.index_of(p.id.as_str())?))))
            .collect();
        values.sort_by(|a, b| a.0.cmp(b.0));
        for (_, v) in values {
            hash.f32(v);
        }
        hash.physics(&physics);
    }
    hash.0
}

/// 渡す `ParameterValues` の並びを途中で変えても、結果は並びによらない (どのプラットフォームでも成り立つ)。
#[test]
fn reordered_parameters_give_the_same_result() {
    for (settings, model) in [
        (physics_hair_settings(), physics_hair as fn() -> Model),
        (physics_relay_settings(), physics_relay),
    ] {
        assert_eq!(
            run_update(settings.clone(), &[model(), reversed(model())]),
            run_update(settings, &[model()]),
        );
    }
}

// 記録したハッシュは、CI と同じプラットフォーム (x86_64 Linux) でだけ比べる。
// T0180 で数学関数を `ugoe_core::math` (常に libm) にしたとき、`sin`・`atan2`・`powf` などの最後の桁が変わったので記録し直した
// (入力の `sin`・`cos` も libm になった。見本のモデルと物理のフィクスチャで 900 フレームずつ比べた
// 前後の差は、パラメータと頂点の値 261 万個のうち 401 個が変わり、最大で 6.1e-5)。
// libm にしたので OS の数学ライブラリの差は無くなったが、ほかのプラットフォームで同じハッシュになるかは確かめていない。

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn hair_update_matches_recorded_hash() {
    let hash = run_update(physics_hair_settings(), &[physics_hair()]);
    assert_eq!(hash, 0x5231_b2df_e2dd_5849, "{hash:#x}");
}

/// 並びが変わる・パラメータが足りない `ParameterValues` に切り替える。
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn hair_update_with_changing_layouts_matches_recorded_hash() {
    let hash = run_update(
        physics_hair_settings(),
        &[physics_hair(), reversed(physics_hair()), physics_relay()],
    );
    assert_eq!(hash, 0x061a_b7ae_3da1_4da6, "{hash:#x}");
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn relay_update_matches_recorded_hash() {
    let hash = run_update(physics_relay_settings(), &[physics_relay()]);
    assert_eq!(hash, 0x6b47_ebf2_cbb6_b40f, "{hash:#x}");
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn dynamic_update_instance_matches_recorded_hash() {
    let mut physics = PhysicsSystem::new(dynamic_hair_settings());
    let mut instance = ModelInstance::new(CompiledModel::compile(dynamic_hair()).unwrap());
    let mut hash = Hasher::new();
    for k in 0..240 {
        let (x, z, dt) = input(k);
        set(instance.parameters_mut(), ANGLE_X, x);
        set(instance.parameters_mut(), ANGLE_Z, z);
        physics.update_instance(&mut instance, dt);
        instance.update();
        for &v in instance.parameters().as_slice() {
            hash.f32(v);
        }
        for d in instance.drawables() {
            for &p in &d.positions {
                hash.vec2(p);
            }
        }
        hash.physics(&physics);
    }
    assert_eq!(hash.0, 0x5f37_83b1_18a2_e986, "{:#x}", hash.0);
}
