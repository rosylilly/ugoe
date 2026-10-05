//! 公開仕様の例 (`spec/examples/`) を、このランタイムが満たすことの検査。
//! 例の形式と許容誤差は `spec/examples/README.md`。
//!
//! - `model.json` はフィクスチャから `just gen` で作る (`ugoe_model::spec_files`)。
//! - `expected.json` は仕様の式から手で導いた値。このテストの側で値を合わせてはならない。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use ugoe_core::{Rgb, Vec2};
use ugoe_format::model_from_json;
use ugoe_model::validate::validate;
use ugoe_runtime::{CompiledModel, DrawableState, ModelInstance};

/// 頂点の位置の許容誤差 (canvas の px)。`spec/examples/README.md#tolerances` (評価の適合の基準と同じ)。
const POSITION_TOLERANCE: f32 = 0.01;

/// 不透明度・色・マスクの効き具合の許容誤差。
const VALUE_TOLERANCE: f32 = 1e-4;

/// 描画順の値の許容誤差。描画の順番は、値の差がこれ以下のメッシュの間では問わない。
const DRAW_ORDER_TOLERANCE: f32 = 1e-3;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    model: String,
    #[allow(dead_code)]
    summary: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    #[serde(default)]
    parameters: BTreeMap<String, f32>,
    #[serde(default)]
    meshes: BTreeMap<String, ExpectedMesh>,
    render_order: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedMesh {
    positions: Option<Vec<[f32; 2]>>,
    opacity: Option<f32>,
    draw_order: Option<f32>,
    multiply_color: Option<Rgb>,
    screen_color: Option<Rgb>,
    mask_strength: Option<f32>,
}

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/examples")
}

fn example_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(examples_dir())
        .expect("spec/examples")
        .map(|entry| entry.expect("spec/examples").path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

/// 評価の例は `expected.json`、時間の例は `timeline.json`、ファイルの例は `bundles.json`、
/// 描画の例は `render.json` を持つ (時間の例の照合は `crates/ugoe/tests/spec_timelines.rs`、
/// ファイルの例の照合は `tests/spec_bundles.rs`、描画の例の照合は `crates/ugoe-render/tests/spec_render.rs`)。
#[test]
fn every_example_has_a_model_and_expected_values() {
    let dirs = example_dirs();
    assert!(!dirs.is_empty());
    for dir in dirs {
        assert!(
            dir.join("expected.json").is_file()
                || dir.join("timeline.json").is_file()
                || dir.join("bundles.json").is_file()
                || dir.join("render.json").is_file(),
            "{}",
            dir.display()
        );
    }
}

#[test]
fn runtime_meets_the_expected_values_of_every_example() {
    let mut failures = Vec::new();
    for dir in example_dirs()
        .into_iter()
        .filter(|dir| dir.join("expected.json").is_file())
    {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let expected: Expected =
            serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap())
                .unwrap_or_else(|e| panic!("{name}/expected.json: {e}"));
        let model = model_from_json(&std::fs::read_to_string(dir.join(&expected.model)).unwrap())
            .unwrap_or_else(|e| panic!("{name}/{}: {e}", expected.model));
        let errors = validate(&model);
        assert!(errors.is_empty(), "{name}: invalid model: {errors:?}");
        let compiled = CompiledModel::compile(model).unwrap();
        for (i, case) in expected.cases.iter().enumerate() {
            let mut instance = ModelInstance::new(compiled.clone());
            for (id, &value) in &case.parameters {
                let index = instance
                    .parameters()
                    .index_of(id)
                    .unwrap_or_else(|| panic!("{name} case {i}: unknown parameter {id}"));
                instance.parameters_mut().set(index, value);
            }
            instance.update();
            check_case(&instance, case, &format!("{name} case {i}"), &mut failures);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn check_case(instance: &ModelInstance, case: &Case, label: &str, failures: &mut Vec<String>) {
    let model = instance.compiled().model();
    let mesh_index = |id: &str| {
        model
            .meshes
            .iter()
            .position(|m| m.id == id)
            .unwrap_or_else(|| panic!("{label}: unknown mesh {id}"))
    };
    for (id, expected) in &case.meshes {
        let actual = &instance.drawables()[mesh_index(id)];
        check_mesh(actual, expected, &format!("{label} {id}"), failures);
    }
    if let Some(order) = &case.render_order {
        let actual: Vec<&str> = instance
            .render_order()
            .iter()
            .map(|&i| model.meshes[i as usize].id.as_str())
            .collect();
        let draw_order = |id: &str| instance.drawables()[mesh_index(id)].draw_order;
        let mut sorted_actual = actual.clone();
        let mut sorted_expected: Vec<&str> = order.iter().map(String::as_str).collect();
        sorted_actual.sort_unstable();
        sorted_expected.sort_unstable();
        let tied = |(a, e): (&&str, &String)| {
            *a == e.as_str() || (draw_order(a) - draw_order(e)).abs() <= DRAW_ORDER_TOLERANCE
        };
        if sorted_actual != sorted_expected || !actual.iter().zip(order).all(tied) {
            failures.push(format!("{label}: render order {actual:?} != {order:?}"));
        }
    }
}

fn check_mesh(actual: &DrawableState, expected: &ExpectedMesh, label: &str, out: &mut Vec<String>) {
    if let Some(positions) = &expected.positions {
        if actual.positions.len() != positions.len() {
            out.push(format!("{label}: {} vertices", actual.positions.len()));
        }
        for (v, (a, &[x, y])) in actual.positions.iter().zip(positions).enumerate() {
            if !a.abs_diff_eq(Vec2::new(x, y), POSITION_TOLERANCE) {
                out.push(format!("{label} vertex {v}: {a} != ({x}, {y})"));
            }
        }
    }
    if let Some(expected) = expected.draw_order
        && (actual.draw_order - expected).abs() > DRAW_ORDER_TOLERANCE
    {
        out.push(format!(
            "{label} draw_order: {} != {expected}",
            actual.draw_order
        ));
    }
    let mut value = |field: &str, actual: f32, expected: Option<f32>| {
        if let Some(expected) = expected
            && (actual - expected).abs() > VALUE_TOLERANCE
        {
            out.push(format!("{label} {field}: {actual} != {expected}"));
        }
    };
    value("opacity", actual.opacity, expected.opacity);
    value(
        "mask_strength",
        actual.mask_strength,
        expected.mask_strength,
    );
    for (field, actual, expected) in [
        (
            "multiply_color",
            actual.multiply_color,
            expected.multiply_color,
        ),
        ("screen_color", actual.screen_color, expected.screen_color),
    ] {
        if let Some(e) = expected {
            value(&format!("{field}.r"), actual.r, Some(e.r));
            value(&format!("{field}.g"), actual.g, Some(e.g));
            value(&format!("{field}.b"), actual.b, Some(e.b));
        }
    }
}
