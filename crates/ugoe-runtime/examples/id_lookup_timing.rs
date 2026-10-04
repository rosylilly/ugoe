//! ID から添字への引き当てを含む 1 フレームの費用を計測する (T0146)。
//!
//! `cargo run --release -p ugoe-runtime --example id_lookup_timing`
//!
//! 毎フレームの経路で ID を引く呼び出し側 (`ugoe::Puppet` のモーションのパーツの不透明度のカーブ、
//! C ABI の ID によるパラメータの一括設定) を模して、すべてのパーツを `part_index` で、すべての
//! パラメータを `index_of` で引いて書き込み、`ModelInstance::update` で評価する。

use std::hint::black_box;
use std::time::{Duration, Instant};

use ugoe_model::{Model, Parameter, Part, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance};

const RUNS: usize = 101;
const LOOKUP_REPEAT: usize = 100;

fn main() {
    for (params, parts, meshes) in [(20, 20, 20), (300, 300, 200)] {
        report(params, parts, meshes);
    }
}

fn report(params: usize, parts: usize, meshes: usize) {
    let model = model(params, parts, meshes);
    let param_ids: Vec<String> = model.parameters.iter().map(|p| p.id.to_string()).collect();
    let part_ids: Vec<String> = model.parts.iter().map(|p| p.id.to_string()).collect();
    let mut instance = ModelInstance::new(CompiledModel::compile(model).expect("compile"));

    let lookup = |instance: &mut ModelInstance| {
        let compiled = std::sync::Arc::clone(instance.compiled());
        for id in &part_ids {
            let i = compiled.part_index(id).expect("part");
            instance.set_part_opacity_override(i, 1.0, 1.0);
        }
        let values = instance.parameters_mut();
        for id in &param_ids {
            let i = values.index_of(id).expect("parameter");
            values.set(i, 0.5);
        }
    };
    // 引き当ては短いので、LOOKUP_REPEAT 回の平均を 1 回ぶんとする。
    let lookups = measure(|| {
        for _ in 0..LOOKUP_REPEAT {
            lookup(black_box(&mut instance));
        }
    }) / LOOKUP_REPEAT as u32;
    let update = measure(|| instance.update());
    let frame = measure(|| {
        lookup(&mut instance);
        instance.update();
    });

    println!("{params} parameters / {parts} parts / {meshes} meshes (median of {RUNS} runs):");
    println!(
        "  lookups by ID ({:>3} + {:>3})  {:>8.4} ms",
        params,
        parts,
        ms(lookups)
    );
    println!("  update                     {:>8.4} ms", ms(update));
    println!("  lookups + update           {:>8.4} ms", ms(frame));
}

fn measure(mut f: impl FnMut()) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// `single_quad` の四角形を `meshes` 枚並べたモデル。メッシュ `i` はパーツ `i % parts` に属し、
/// パラメータ `i % params` で動く。ID は実際のモデルに近い長さにする。
fn model(params: usize, parts: usize, meshes: usize) -> Model {
    let mut model = fixtures::single_quad();
    let parameter = model.parameters[0].clone();
    let part = model.parts[0].clone();
    let mesh = model.meshes[0].clone();
    let param_id = |i: usize| format!("ParamHairFrontSway{i:03}");
    let part_id = |i: usize| format!("PartHairFrontLayer{i:03}");
    model.parameters = (0..params)
        .map(|i| Parameter {
            id: param_id(i).as_str().into(),
            ..parameter.clone()
        })
        .collect();
    model.parts = (0..parts)
        .map(|i| Part {
            id: part_id(i).as_str().into(),
            ..part.clone()
        })
        .collect();
    model.meshes = (0..meshes)
        .map(|i| {
            let mut m = mesh.clone();
            m.id = format!("Mesh{i:03}").as_str().into();
            m.part = part_id(i % parts).as_str().into();
            m.keyforms.bindings[0].parameter = param_id(i % params).as_str().into();
            m
        })
        .collect();
    model
}
