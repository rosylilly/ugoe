//! 物理の 1 フレームにかかる時間を計測する (T0319)。
//!
//! `cargo run --release -p ugoe-physics --example physics_timing`
//!
//! 前半は物理だけの時間。
//!
//! - `physics_hair`: 中継のない設定を `PhysicsSystem::update` で進める。
//! - `physics_hair + 100 params`: 同じ設定で、モデルに使わないパラメータを 100 個 (先頭に) 足したもの
//!   (パラメータの名前の探索の重さを見る)。
//! - `physics_relay`: 中継のある設定 (前のリグの出力を後のリグが読む) を `PhysicsSystem::update` で進める。
//! - `dynamic_hair`: 動的物理を `PhysicsSystem::update_instance` で進める (1 段目の評価を含む。
//!   2 段目の `ModelInstance::update` は含めない)。
//!
//! 後半は `dynamic_hair` の段の内訳 (T0334。docs/testing.md#1-フレームの評価)。動的物理の 1 フレームは
//! 1 段目の評価 → 物理 → 2 段目の評価と進むので、それぞれを分けて測る。
//!
//! - `first stage`: 1 段目の評価 (`ModelInstance::update_first_stage`) だけ。`update_instance` の中で行うものと同じ処理を、
//!   別の回で `update_instance` の前に呼んで測る (その回の `update_instance` と `update` は測らない)。
//! - `update_instance`: `PhysicsSystem::update_instance` の 1 回 (1 段目の評価を含む)。
//! - `physics itself`: `update_instance` と `first stage` の中央値の差 (鎖の積分・衝突・出力。最も速いものの列も同じく差)。
//! - `second stage`: 続く `ModelInstance::update` (2 段目の評価)。
//! - `frame`: `update_instance` と `update` を合わせた 1 フレーム。
//!
//! 中程度のモデルを模した生成のモデルと見本の `avatar.ugoe` の段の内訳は、`ugoe` の例 `puppet_timing` が同じ行で出す
//! (`ugoe-physics` は `.ugoe` を読めない。`ugoe-format` に dev で依存すると crates.io に公開する順が循環する)。
//!
//! 表示のフレームレートを 60fps、物理のステップを設定の `fps` (既定 60) とし、入力を毎フレーム動かす。
//! ほかの処理の揺れを除くため、`BATCH` フレームずつの平均を取り、そのうち最も速いものと中央値を示す。

use std::hint::black_box;
use std::time::{Duration, Instant};

use ugoe_core::math;
use ugoe_model::fixtures;
use ugoe_model::standard::{ANGLE_X, ANGLE_Z, BODY_ANGLE_X};
use ugoe_physics::PhysicsSystem;
use ugoe_runtime::{CompiledModel, ModelInstance, ParameterValues};

const FRAMES: usize = 20_000;
const BATCH: usize = 1_000;
const DT: f32 = 1.0 / 60.0;

fn main() {
    println!("per frame (us), batches of {BATCH} frames out of {FRAMES}: best / median");
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

    println!();
    println!("dynamic_hair stages per frame (us): best / median");
    measure_stages();
}

/// `BATCH` フレームずつの 1 フレームあたりの時間。
#[derive(Default)]
struct Batches {
    total: Duration,
    frames: usize,
    averages: Vec<Duration>,
}

impl Batches {
    fn add(&mut self, elapsed: Duration) {
        self.total += elapsed;
        self.frames += 1;
        if self.frames == BATCH {
            self.averages.push(self.total / BATCH as u32);
            self.total = Duration::ZERO;
            self.frames = 0;
        }
    }

    /// (最も速いもの, 中央値)。
    fn summary(mut self) -> (Duration, Duration) {
        self.averages.sort();
        (self.averages[0], self.averages[self.averages.len() / 2])
    }
}

fn us(d: Duration) -> f64 {
    d.as_secs_f64() * 1e6
}

fn report(name: &str, (best, median): (Duration, Duration)) {
    println!("  {name:32} {:>8.2} / {:>8.2}", us(best), us(median));
}

/// フレーム `k` の入力の値。
fn wave(k: usize) -> (f32, f32) {
    (
        30.0 * math::sin(k as f32 * 0.07),
        10.0 * math::cos(k as f32 * 0.03),
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
) -> (Duration, Duration) {
    let mut physics = PhysicsSystem::new(settings);
    let mut params = ParameterValues::from_model(model);
    let mut batches = Batches::default();
    for k in 0..FRAMES {
        let (x, z) = wave(k);
        set(&mut params, ANGLE_X, x);
        set(&mut params, BODY_ANGLE_X, z);
        let start = Instant::now();
        physics.update(&mut params, DT);
        batches.add(start.elapsed());
        black_box(params.as_slice());
    }
    batches.summary()
}

/// `dynamic_hair` を物理に結び付けたもの。
fn dynamic_setup() -> (ModelInstance, PhysicsSystem) {
    let compiled = CompiledModel::compile(fixtures::dynamic_hair()).expect("compile");
    let mut instance = ModelInstance::new(compiled);
    let mut physics = PhysicsSystem::new(fixtures::dynamic_hair_settings());
    physics.bind(&mut instance);
    (instance, physics)
}

/// フレーム `k` の動的物理の入力を書く。
fn drive(instance: &mut ModelInstance, k: usize) {
    let (x, z) = wave(k);
    set(instance.parameters_mut(), ANGLE_X, x);
    set(instance.parameters_mut(), ANGLE_Z, z);
}

fn measure_dynamic() -> (Duration, Duration) {
    let (mut instance, mut physics) = dynamic_setup();
    let mut batches = Batches::default();
    for k in 0..FRAMES {
        drive(&mut instance, k);
        let start = Instant::now();
        physics.update_instance(&mut instance, DT);
        batches.add(start.elapsed());
        instance.update();
        black_box(instance.drawables());
    }
    batches.summary()
}

/// `dynamic_hair` の段の内訳 (先頭のコメントの後半)。
fn measure_stages() {
    let (mut instance, mut physics) = dynamic_setup();
    let mut physics_stage = Batches::default();
    let mut second = Batches::default();
    let mut frame = Batches::default();
    for k in 0..FRAMES {
        drive(&mut instance, k);
        let start = Instant::now();
        physics.update_instance(&mut instance, DT);
        let middle = Instant::now();
        instance.update();
        let end = Instant::now();
        physics_stage.add(middle - start);
        second.add(end - middle);
        frame.add(end - start);
        black_box(instance.drawables());
    }

    // 1 段目だけを別の回で測る。物理と 2 段目は同じに進め (状態を本番と揃える)、時間は測らない。
    let (mut instance, mut physics) = dynamic_setup();
    let mut first = Batches::default();
    for k in 0..FRAMES {
        drive(&mut instance, k);
        let start = Instant::now();
        instance.update_first_stage();
        first.add(start.elapsed());
        physics.update_instance(&mut instance, DT);
        instance.update();
        black_box(instance.drawables());
    }

    let first = first.summary();
    let physics_stage = physics_stage.summary();
    report("first stage", first);
    report("update_instance", physics_stage);
    report(
        "physics itself (difference)",
        (
            physics_stage.0.saturating_sub(first.0),
            physics_stage.1.saturating_sub(first.1),
        ),
    );
    report("second stage (update)", second.summary());
    report("frame", frame.summary());
}
