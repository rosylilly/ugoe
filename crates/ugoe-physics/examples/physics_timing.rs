//! 物理の 1 フレームにかかる時間を計測する (T0319)。
//!
//! `cargo run --release -p ugoe-physics --example physics_timing`
//!
//! - `physics_hair`: 中継のない設定を `PhysicsSystem::update` で進める。
//! - `physics_hair + 100 params`: 同じ設定で、モデルに使わないパラメータを 100 個 (先頭に) 足したもの
//!   (パラメータの名前の探索の重さを見る)。
//! - `physics_relay`: 中継のある設定 (前のリグの出力を後のリグが読む) を `PhysicsSystem::update` で進める。
//! - `dynamic_hair`: 動的物理を `PhysicsSystem::update_instance` で進める (1 段目の評価を含む。
//!   2 段目の `ModelInstance::update` は含めない)。
//!
//! 表示のフレームレートを 60fps、物理のステップを設定の `fps` (既定 60) とし、入力を毎フレーム動かす。
//! ほかの処理の揺れを除くため、`BATCH` フレームずつの平均のうち最も速いものを示す。

use std::hint::black_box;
use std::time::{Duration, Instant};

use ugoe_model::fixtures;
use ugoe_model::standard::{ANGLE_X, ANGLE_Z, BODY_ANGLE_X};
use ugoe_physics::PhysicsSystem;
use ugoe_runtime::{CompiledModel, ModelInstance, ParameterValues};

const FRAMES: usize = 20_000;
const BATCH: usize = 1_000;
const DT: f32 = 1.0 / 60.0;

fn main() {
    let hair = measure_update(fixtures::physics_hair_settings(), &fixtures::physics_hair());
    report("physics_hair (update)", hair);
    let mut padded = fixtures::physics_hair();
    let extra: Vec<_> = (0..100)
        .map(|i| {
            let mut p = padded.parameters[0].clone();
            p.id = format!("Unused{i:03}").as_str().into();
            p
        })
        .collect();
    padded.parameters.splice(0..0, extra);
    let padded = measure_update(fixtures::physics_hair_settings(), &padded);
    report("physics_hair + 100 params", padded);
    let relay = measure_update(
        fixtures::physics_relay_settings(),
        &fixtures::physics_relay(),
    );
    report("physics_relay (update)", relay);
    report("dynamic_hair (update_instance)", measure_dynamic());
}

/// `BATCH` フレームずつの 1 フレームあたりの時間の最小値。
struct Batches {
    total: Duration,
    frames: usize,
    best: Duration,
}

impl Default for Batches {
    fn default() -> Self {
        Self {
            total: Duration::ZERO,
            frames: 0,
            best: Duration::MAX,
        }
    }
}

impl Batches {
    fn add(&mut self, elapsed: Duration) {
        self.total += elapsed;
        self.frames += 1;
        if self.frames == BATCH {
            self.best = self.best.min(self.total / BATCH as u32);
            self.total = Duration::ZERO;
            self.frames = 0;
        }
    }
}

fn report(name: &str, per_frame: Duration) {
    println!("{name:32} {:>8.2} us/frame", per_frame.as_secs_f64() * 1e6);
}

/// フレーム `k` の入力の値。
fn wave(k: usize) -> (f32, f32) {
    (
        30.0 * (k as f32 * 0.07).sin(),
        10.0 * (k as f32 * 0.03).cos(),
    )
}

fn set(params: &mut ParameterValues, id: &str, value: f32) {
    if let Some(index) = params.index_of(id) {
        params.set(index, value);
    }
}

fn measure_update(
    settings: ugoe_model::physics::PhysicsSettings,
    model: &ugoe_model::Model,
) -> Duration {
    let mut physics = PhysicsSystem::new(settings);
    let mut params = ParameterValues::from_model(model);
    let mut best = Batches::default();
    for k in 0..FRAMES {
        let (x, z) = wave(k);
        set(&mut params, ANGLE_X, x);
        set(&mut params, BODY_ANGLE_X, z);
        let start = Instant::now();
        physics.update(&mut params, DT);
        best.add(start.elapsed());
        black_box(params.as_slice());
    }
    best.best
}

fn measure_dynamic() -> Duration {
    let compiled = CompiledModel::compile(fixtures::dynamic_hair()).expect("compile");
    let mut instance = ModelInstance::new(compiled);
    let mut physics = PhysicsSystem::new(fixtures::dynamic_hair_settings());
    physics.bind(&mut instance);
    let mut best = Batches::default();
    for k in 0..FRAMES {
        let (x, z) = wave(k);
        set(instance.parameters_mut(), ANGLE_X, x);
        set(instance.parameters_mut(), ANGLE_Z, z);
        let start = Instant::now();
        physics.update_instance(&mut instance, DT);
        best.add(start.elapsed());
        instance.update();
        black_box(instance.drawables());
    }
    best.best
}
