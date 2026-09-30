//! 公開仕様の時間の例 (`spec/examples/*/timeline.json`) を、参照実装の [`Puppet`] が満たすことの検査。
//! 例の形式と許容誤差は `spec/examples/README.md#timeline-examples`。
//!
//! 期待値は仕様の文から導いた値。このテストの側で値や許容誤差を合わせてはならない。
//! 食い違う例は [`KNOWN_MISMATCHES`] に理由を付けて入れる。

#![allow(clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use ugoe::Puppet;
use ugoe::core::{Rgb, Vec2};
use ugoe::format::{expression_from_json, model_from_json, motion_from_json, physics_from_json};
use ugoe::model::expression::Expression;
use ugoe::model::motion::Motion;
use ugoe::model::physics::PhysicsTarget;
use ugoe::model::validate::validate;
use ugoe::motion::Priority;
use ugoe::runtime::DrawableState;

/// 参照実装と期待値が食い違う例 (例のディレクトリ名 → 理由)。ここにある例は照合しても失敗にしない。
const KNOWN_MISMATCHES: &[(&str, &str)] = &[];

/// 物理の出力先のパラメータの許容誤差の係数 (`× (max − min)`)。`docs/spec/physics.md` の適合の基準。
const PHYSICS_PARAMETER_TOLERANCE: f32 = 1e-3;
/// そのほかのパラメータの許容誤差の係数 (`× max(1, max − min)`)。`docs/spec/motion.md` の適合の基準。
const PARAMETER_TOLERANCE: f32 = 1e-4;
/// パーツの不透明度の上書き (値と重み)、メッシュの不透明度・色・マスクの効き具合の許容誤差。
const VALUE_TOLERANCE: f32 = 1e-4;
/// 動的物理の角度の加算量の許容誤差 (度)。
const ANGLE_TOLERANCE: f32 = 0.05;
/// 動的物理の位置の加算量とメッシュの頂点の位置の許容誤差 (canvas の px)。
const POSITION_TOLERANCE: f32 = 0.05;
/// 描画順の値の許容誤差。
const DRAW_ORDER_TOLERANCE: f32 = 1e-3;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Timeline {
    model: String,
    physics: Option<String>,
    #[serde(default)]
    motions: BTreeMap<String, String>,
    #[serde(default)]
    expressions: BTreeMap<String, String>,
    #[allow(dead_code)]
    summary: String,
    frames: Vec<Frame>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    #[serde(default)]
    actions: Vec<Action>,
    dt: f32,
    #[serde(default)]
    expect: Expect,
}

#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum Action {
    Set {
        set: BTreeMap<String, f32>,
    },
    StartMotion {
        start_motion: String,
        priority: PriorityName,
        accepted: Option<bool>,
    },
    StopMotions {
        #[allow(dead_code)]
        stop_motions: bool,
    },
    SetExpression {
        set_expression: String,
    },
    AddExpression {
        add_expression: String,
    },
    StopExpressions {
        #[allow(dead_code)]
        stop_expressions: bool,
    },
    LipSync {
        lip_sync: Option<f32>,
    },
    ResetPhysics {
        #[allow(dead_code)]
        reset_physics: bool,
    },
    Stages {
        stages: BTreeMap<String, bool>,
    },
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum PriorityName {
    Idle,
    Normal,
    Force,
}

impl From<PriorityName> for Priority {
    fn from(p: PriorityName) -> Self {
        match p {
            PriorityName::Idle => Priority::Idle,
            PriorityName::Normal => Priority::Normal,
            PriorityName::Force => Priority::Force,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Expect {
    #[serde(default)]
    parameters: BTreeMap<String, f32>,
    #[serde(default)]
    part_opacity: BTreeMap<String, PartOpacity>,
    events: Option<Vec<String>>,
    offsets: Option<Offsets>,
    #[serde(default)]
    meshes: BTreeMap<String, ExpectedMesh>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartOpacity {
    value: f32,
    weight: f32,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Offsets {
    #[serde(default)]
    deformer_angles: BTreeMap<String, f32>,
    #[serde(default)]
    deformer_origins: BTreeMap<String, [f32; 2]>,
    #[serde(default)]
    warp_points: BTreeMap<String, Vec<[f32; 2]>>,
    #[serde(default)]
    mesh_vertices: BTreeMap<String, Vec<[f32; 2]>>,
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

fn timeline_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(examples_dir())
        .expect("spec/examples")
        .map(|entry| entry.expect("spec/examples").path())
        .filter(|path| path.join("timeline.json").is_file())
        .collect();
    dirs.sort();
    dirs
}

fn read(dir: &Path, file: &str) -> String {
    std::fs::read_to_string(dir.join(file))
        .unwrap_or_else(|e| panic!("{}/{file}: {e}", dir.display()))
}

#[test]
fn puppet_meets_the_expected_values_of_every_timeline_example() {
    let dirs = timeline_dirs();
    assert!(!dirs.is_empty());
    let mut failures = Vec::new();
    let mut known = Vec::new();
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let mut example_failures = Vec::new();
        run_timeline(&dir, &name, &mut example_failures);
        if KNOWN_MISMATCHES.iter().any(|(n, _)| *n == name) {
            known.push(format!(
                "{name}: {} mismatches (known)",
                example_failures.len()
            ));
        } else {
            failures.extend(example_failures);
        }
    }
    for line in &known {
        eprintln!("{line}");
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// 一覧に入れた例が、今は食い違っていないなら知らせる (一覧から外すため)。
#[test]
fn known_mismatches_still_mismatch() {
    for (name, _) in KNOWN_MISMATCHES {
        let dir = examples_dir().join(name);
        let mut failures = Vec::new();
        run_timeline(&dir, name, &mut failures);
        assert!(!failures.is_empty(), "{name} now matches");
    }
}

fn run_timeline(dir: &Path, name: &str, failures: &mut Vec<String>) {
    let timeline: Timeline = serde_json::from_str(&read(dir, "timeline.json"))
        .unwrap_or_else(|e| panic!("{name}/timeline.json: {e}"));
    let model = model_from_json(&read(dir, &timeline.model))
        .unwrap_or_else(|e| panic!("{name}/{}: {e}", timeline.model));
    let errors = validate(&model);
    assert!(errors.is_empty(), "{name}: invalid model: {errors:?}");
    let motions: BTreeMap<&str, Motion> = timeline
        .motions
        .iter()
        .map(|(key, file)| {
            let motion =
                motion_from_json(&read(dir, file)).unwrap_or_else(|e| panic!("{name}/{file}: {e}"));
            (key.as_str(), motion)
        })
        .collect();
    let expressions: BTreeMap<&str, Expression> = timeline
        .expressions
        .iter()
        .map(|(key, file)| {
            let expression = expression_from_json(&read(dir, file))
                .unwrap_or_else(|e| panic!("{name}/{file}: {e}"));
            (key.as_str(), expression)
        })
        .collect();

    let mut puppet = Puppet::new(model).unwrap();
    // 物理の出力先のパラメータ (許容誤差が違う)。
    let mut physics_targets = BTreeSet::new();
    if let Some(file) = &timeline.physics {
        let settings =
            physics_from_json(&read(dir, file)).unwrap_or_else(|e| panic!("{name}/{file}: {e}"));
        for output in settings.rigs.iter().flat_map(|r| &r.outputs) {
            if let PhysicsTarget::Parameter { id } = &output.target {
                physics_targets.insert(id.as_str().to_owned());
            }
        }
        puppet
            .set_physics(Some(settings))
            .unwrap_or_else(|e| panic!("{name}/{file}: {e:?}"));
    }

    for (f, frame) in timeline.frames.iter().enumerate() {
        let label = format!("{name} frame {f}");
        for action in &frame.actions {
            perform(
                &mut puppet,
                action,
                &motions,
                &expressions,
                &label,
                failures,
            );
        }
        puppet.update(frame.dt);
        check_frame(&puppet, &frame.expect, &physics_targets, &label, failures);
    }
}

fn perform(
    puppet: &mut Puppet,
    action: &Action,
    motions: &BTreeMap<&str, Motion>,
    expressions: &BTreeMap<&str, Expression>,
    label: &str,
    failures: &mut Vec<String>,
) {
    match action {
        Action::Set { set } => {
            let params = puppet.instance_mut().parameters_mut();
            for (id, &value) in set {
                let index = params
                    .index_of(id)
                    .unwrap_or_else(|| panic!("{label}: unknown parameter {id}"));
                params.set(index, value);
            }
        }
        Action::StartMotion {
            start_motion,
            priority,
            accepted,
        } => {
            let motion = motions
                .get(start_motion.as_str())
                .unwrap_or_else(|| panic!("{label}: unknown motion {start_motion}"))
                .clone();
            let actual = puppet.start_motion(motion, (*priority).into());
            if let Some(expected) = accepted
                && actual != *expected
            {
                failures.push(format!(
                    "{label} start_motion {start_motion}: accepted {actual} != {expected}"
                ));
            }
        }
        Action::StopMotions { .. } => puppet.motions_mut().stop_all(),
        Action::SetExpression { set_expression } => {
            puppet.set_expression(expression(expressions, set_expression, label));
        }
        Action::AddExpression { add_expression } => {
            let expression = expression(expressions, add_expression, label);
            puppet.expressions_mut().add(expression);
        }
        Action::StopExpressions { .. } => puppet.expressions_mut().stop_all(),
        Action::LipSync { lip_sync } => puppet.set_lip_sync_level(*lip_sync),
        Action::ResetPhysics { .. } => puppet.reset_physics(),
        Action::Stages { stages } => {
            let mut toggles = puppet.toggles();
            for (stage, &on) in stages {
                let field = match stage.as_str() {
                    "physics" => &mut toggles.physics,
                    "eye_blink" => &mut toggles.eye_blink,
                    "breath" => &mut toggles.breath,
                    "lip_sync" => &mut toggles.lip_sync,
                    "pose" => &mut toggles.pose,
                    _ => panic!("{label}: unknown stage {stage}"),
                };
                *field = on;
            }
            puppet.set_toggles(toggles);
        }
    }
}

fn expression(expressions: &BTreeMap<&str, Expression>, key: &str, label: &str) -> Expression {
    expressions
        .get(key)
        .unwrap_or_else(|| panic!("{label}: unknown expression {key}"))
        .clone()
}

fn check_frame(
    puppet: &Puppet,
    expect: &Expect,
    physics_targets: &BTreeSet<String>,
    label: &str,
    failures: &mut Vec<String>,
) {
    let instance = puppet.instance();
    let compiled = instance.compiled();
    let model = compiled.model();

    for (id, &expected) in &expect.parameters {
        let params = instance.parameters();
        let index = params
            .index_of(id)
            .unwrap_or_else(|| panic!("{label}: unknown parameter {id}"));
        let parameter = &model.parameters[index.0 as usize];
        let range = parameter.max - parameter.min;
        let tolerance = if physics_targets.contains(id) {
            PHYSICS_PARAMETER_TOLERANCE * range
        } else {
            PARAMETER_TOLERANCE * range.max(1.0)
        };
        let actual = params.get(index);
        if (actual - expected).abs() > tolerance {
            failures.push(format!("{label} parameter {id}: {actual} != {expected}"));
        }
    }

    for (id, expected) in &expect.part_opacity {
        let part = compiled
            .part_index(id)
            .unwrap_or_else(|| panic!("{label}: unknown part {id}"));
        let (value, weight) = instance.part_opacity_override(part);
        let weight_ok = (weight - expected.weight).abs() <= VALUE_TOLERANCE;
        let value_ok = expected.weight == 0.0 || (value - expected.value).abs() <= VALUE_TOLERANCE;
        if !weight_ok || !value_ok {
            failures.push(format!(
                "{label} part_opacity {id}: (value {value}, weight {weight}) != (value {}, weight {})",
                expected.value, expected.weight
            ));
        }
    }

    if let Some(expected) = &expect.events {
        let actual: Vec<&str> = puppet
            .motion_events()
            .iter()
            .map(|e| e.value.as_str())
            .collect();
        if actual != *expected {
            failures.push(format!("{label} events: {actual:?} != {expected:?}"));
        }
    }

    if let Some(offsets) = &expect.offsets {
        check_offsets(puppet, offsets, label, failures);
    }

    for (id, expected) in &expect.meshes {
        let mesh = compiled
            .mesh_index(id)
            .unwrap_or_else(|| panic!("{label}: unknown mesh {id}"));
        check_mesh(
            &instance.drawables()[mesh],
            expected,
            &format!("{label} mesh {id}"),
            failures,
        );
    }
}

/// 動的物理の加算量。加算量のバッファが無い (動的物理を使っていない) ときや、まだ確保していない
/// ワープ・メッシュの加算量は 0 とみなす (`spec/examples/README.md`: absent offsets are 0)。
fn check_offsets(puppet: &Puppet, expected: &Offsets, label: &str, failures: &mut Vec<String>) {
    let instance = puppet.instance();
    let compiled = instance.compiled();
    let offsets = instance.dynamic_offsets();
    let deformer = |id: &str| {
        compiled
            .deformer_index(id)
            .unwrap_or_else(|| panic!("{label}: unknown deformer {id}"))
    };
    let points = |actual: &[Vec2], expected: &[[f32; 2]], what: &str, out: &mut Vec<String>| {
        for (i, &[x, y]) in expected.iter().enumerate() {
            let a = actual.get(i).copied().unwrap_or(Vec2::ZERO);
            if !a.abs_diff_eq(Vec2::new(x, y), POSITION_TOLERANCE) {
                out.push(format!("{label} {what} point {i}: {a} != ({x}, {y})"));
            }
        }
    };

    for (id, &expected) in &expected.deformer_angles {
        let actual = offsets.map_or(0.0, |o| o.angle(deformer(id)));
        if (actual - expected).abs() > ANGLE_TOLERANCE {
            failures.push(format!(
                "{label} offsets deformer_angles {id}: {actual} != {expected}"
            ));
        }
    }
    for (id, &[x, y]) in &expected.deformer_origins {
        let actual = offsets.map_or(Vec2::ZERO, |o| o.origin(deformer(id)));
        if !actual.abs_diff_eq(Vec2::new(x, y), POSITION_TOLERANCE) {
            failures.push(format!(
                "{label} offsets deformer_origins {id}: {actual} != ({x}, {y})"
            ));
        }
    }
    for (id, expected) in &expected.warp_points {
        let actual = offsets.map_or(&[][..], |o| o.warp_points(deformer(id)));
        points(
            actual,
            expected,
            &format!("offsets warp_points {id}"),
            failures,
        );
    }
    for (id, expected) in &expected.mesh_vertices {
        let mesh = compiled
            .mesh_index(id)
            .unwrap_or_else(|| panic!("{label}: unknown mesh {id}"));
        let actual = offsets.map_or(&[][..], |o| o.mesh_positions(mesh));
        points(
            actual,
            expected,
            &format!("offsets mesh_vertices {id}"),
            failures,
        );
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
