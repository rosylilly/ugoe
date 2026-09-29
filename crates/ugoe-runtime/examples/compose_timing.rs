//! 着せ替えの合成とコンパイルにかかる時間を計測する (T0111、`docs/spec/modules.md#性能`)。
//!
//! `cargo run --release -p ugoe-runtime --example compose_timing`
//!
//! 中規模のモデルを模した本体 (メッシュ 200 枚・頂点各 64・ワープ 40 個・キーフォーム 9 つ) に、
//! モジュール 4 つ (各メッシュ 16 枚・ワープ 4 個) を取り付ける。

use std::time::{Duration, Instant};

use ugoe_core::{Rgb, Vec2};
use ugoe_model::deformer::{DeformerKind, WarpDeformer, WarpKeyform};
use ugoe_model::mesh::{BlendMode, MaskMode, MeshKeyform};
use ugoe_model::module::{DrawOrderRange, ModulePhysics, ModulePose, ModuleRequirements};
use ugoe_model::parameter::ParameterKind;
use ugoe_model::part::PartKeyform;
use ugoe_model::{
    Deformer, KeyformBinding, KeyformGrid, Mesh, Model, Module, Parameter, Part, SCHEMA_VERSION,
    Slot, TextureRef, fixtures, standard,
};
use ugoe_runtime::{CompiledModel, ModelInstance, compose};

const PARAMS: [&str; 6] = [
    standard::ANGLE_X,
    standard::ANGLE_Y,
    standard::ANGLE_Z,
    standard::BODY_ANGLE_X,
    standard::EYE_L_OPEN,
    standard::MOUTH_OPEN_Y,
];
const SLOTS: [&str; 4] = ["Hair", "Outfit", "Hat", "Shoes"];
const RUNS: usize = 30;

fn main() {
    let base = base_model(200, 40);
    let modules: Vec<Module> = SLOTS.iter().map(|slot| module(slot, 16, 4)).collect();
    let modules: Vec<&Module> = modules.iter().collect();

    let composed = compose(&base, &modules).expect("compose");
    let vertices = |m: &Model| m.meshes.iter().map(|m| m.uvs.len()).sum::<usize>();
    println!(
        "base: {} meshes / {} deformers / {} vertices; composed: {} meshes / {} deformers / {} vertices",
        base.meshes.len(),
        base.deformers.len(),
        vertices(&base),
        composed.meshes.len(),
        composed.deformers.len(),
        vertices(&composed),
    );

    let validate = measure(|| {
        assert!(ugoe_model::validate::validate(&base).is_empty());
    });
    let compose_time = measure(|| {
        compose(&base, &modules).expect("compose");
    });
    let compile = measure(|| {
        CompiledModel::compile(composed.clone()).expect("compile");
    });
    let clone = measure(|| {
        let _ = composed.clone();
    });
    let total = measure(|| {
        let model = compose(&base, &modules).expect("compose");
        let mut instance = ModelInstance::new(CompiledModel::compile(model).expect("compile"));
        instance.update();
    });
    let compiled = CompiledModel::compile(composed.clone()).expect("compile");
    let mut instance = ModelInstance::new(compiled);
    let update = measure(|| instance.update());

    println!("median of {RUNS} runs:");
    println!("  validate (base only)      {:>8.3} ms", ms(validate));
    println!("  compose (incl. validate)  {:>8.3} ms", ms(compose_time));
    println!("  compile (incl. validate)  {:>8.3} ms", ms(compile));
    println!("    of which model clone    {:>8.3} ms", ms(clone));
    println!("  compose+compile+update    {:>8.3} ms", ms(total));
    println!("  update (per frame)        {:>8.3} ms", ms(update));
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

/// 2 軸 × キー 3 つ (9 セル) の格子。
fn grid<T>(cell: impl Fn(f32, f32) -> T, a: &str, b: &str) -> KeyformGrid<T> {
    let keys = [-1.0, 0.0, 1.0];
    let bindings = [a, b]
        .iter()
        .map(|p| KeyformBinding {
            parameter: (*p).into(),
            keys: keys.to_vec(),
        })
        .collect();
    let cells = keys
        .iter()
        .flat_map(|&y| keys.iter().map(move |&x| (x, y)))
        .map(|(x, y)| cell(x, y))
        .collect();
    KeyformGrid::new(bindings, cells)
}

/// 4×4 セルのワープ。制御点は親の UV 空間の `[0.1, 0.9]²`。
fn warp(id: &str, part: &str, parent: Option<&str>, params: (&str, &str)) -> Deformer {
    let points = |dx: f32, dy: f32| {
        (0..5)
            .flat_map(|y| (0..5).map(move |x| (x, y)))
            .map(|(x, y)| {
                Vec2::new(
                    0.1 + 0.2 * x as f32 + 0.01 * dx,
                    0.1 + 0.2 * y as f32 + 0.01 * dy,
                )
            })
            .collect()
    };
    Deformer {
        id: id.into(),
        name: id.into(),
        part: part.into(),
        parent: parent.map(Into::into),
        kind: DeformerKind::Warp(WarpDeformer {
            rows: 4,
            cols: 4,
            keyforms: grid(
                |x, y| WarpKeyform {
                    points: points(x, y),
                    opacity: 1.0,
                },
                params.0,
                params.1,
            ),
            depth: None,
        }),
        mirror: None,
        mirror_axis: None,
    }
}

/// 8×8 頂点 (三角形 98 枚) のメッシュ。
fn mesh(id: &str, part: &str, parent: Option<&str>, params: (&str, &str), order: f32) -> Mesh {
    let n = 8u16;
    let uvs: Vec<Vec2> = (0..n)
        .flat_map(|y| (0..n).map(move |x| (x, y)))
        .map(|(x, y)| Vec2::new(f32::from(x) / 7.0, f32::from(y) / 7.0))
        .collect();
    let mut indices = Vec::new();
    for y in 0..n - 1 {
        for x in 0..n - 1 {
            let i = y * n + x;
            indices.extend([i, i + 1, i + n, i + 1, i + n + 1, i + n]);
        }
    }
    let keyforms = grid(
        |dx, dy| MeshKeyform {
            positions: uvs
                .iter()
                .map(|uv| Vec2::new(0.2 + 0.6 * uv.x + 0.01 * dx, 0.2 + 0.6 * uv.y + 0.01 * dy))
                .collect(),
            opacity: 1.0,
            draw_order: order,
            multiply_color: Rgb::WHITE,
            screen_color: Rgb::BLACK,
            mask_strength: 1.0,
        },
        params.0,
        params.1,
    );
    Mesh {
        id: id.into(),
        name: id.into(),
        part: part.into(),
        parent: parent.map(Into::into),
        texture: 0,
        uvs,
        indices,
        blend: BlendMode::Normal,
        culling: false,
        masks: Vec::new(),
        mask_mode: MaskMode::Union,
        mask_only: false,
        keyforms,
        mirror: None,
        mirror_axis: None,
        depth: None,
        lit: true,
    }
}

fn params_for(i: usize) -> (&'static str, &'static str) {
    (PARAMS[i % PARAMS.len()], PARAMS[(i + 1) % PARAMS.len()])
}

fn part(id: &str) -> Part {
    Part {
        id: id.into(),
        name: id.into(),
        parent: None,
        keyforms: KeyformGrid::constant(PartKeyform { opacity: 1.0 }),
    }
}

fn base_model(meshes: usize, warps: usize) -> Model {
    let mut model = fixtures::dress_up_base();
    model.slots.clear();
    model.parameters = PARAMS
        .iter()
        .map(|&id| {
            let (min, default, max) = standard::range(id).expect("standard");
            Parameter {
                id: id.into(),
                name: id.into(),
                min,
                max,
                default,
                kind: ParameterKind::Normal,
                mirror: None,
                group: None,
                internal: false,
            }
        })
        .collect();
    model.parts = vec![part("Root")];
    model.deformers.clear();
    model.meshes.clear();
    // ワープは 4 段の木 (最上位 → 子 → 孫 → ひ孫) にする。
    for i in 0..warps {
        let parent = (i >= 4).then(|| format!("W{}", i / 4 - 1));
        model.deformers.push(warp(
            &format!("W{i}"),
            "Root",
            parent.as_deref(),
            params_for(i),
        ));
    }
    for i in 0..meshes {
        let parent = format!("W{}", i % warps);
        model.meshes.push(mesh(
            &format!("M{i}"),
            "Root",
            Some(&parent),
            params_for(i),
            (i * 5 % 1000) as f32,
        ));
    }
    model.slots = SLOTS
        .iter()
        .enumerate()
        .map(|(i, slot)| Slot {
            id: (*slot).into(),
            anchor: Some(format!("W{i}").as_str().into()),
            draw_order: DrawOrderRange {
                min: 100.0 * i as f32,
                max: 100.0 * i as f32 + 100.0,
            },
            replaces: Vec::new(),
            parameters: Vec::new(),
            mask_sources: Vec::new(),
            part: None,
            colliders: Vec::new(),
            pose_parts: Vec::new(),
        })
        .collect();
    model
}

fn module(slot: &str, meshes: usize, warps: usize) -> Module {
    let deformers = (0..warps)
        .map(|i| {
            let parent = (i > 0).then(|| "W0".to_owned());
            warp(&format!("W{i}"), "Root", parent.as_deref(), params_for(i))
        })
        .collect();
    let meshes = (0..meshes)
        .map(|i| {
            let parent = format!("W{}", i % warps);
            mesh(
                &format!("M{i}"),
                "Root",
                Some(&parent),
                params_for(i),
                (i * 60 % 1000) as f32,
            )
        })
        .collect();
    Module {
        schema_version: SCHEMA_VERSION,
        name: slot.to_owned(),
        slot: slot.into(),
        requires: ModuleRequirements::default(),
        parts: vec![part("Root")],
        deformers,
        meshes,
        glues: Vec::new(),
        textures: vec![TextureRef {
            name: slot.to_owned(),
            width: 1,
            height: 1,
            normal_map: None,
        }],
        physics: ModulePhysics::default(),
        pose: ModulePose::default(),
    }
}
