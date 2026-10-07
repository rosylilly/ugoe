//! 親のワープからのはみ出しの検出のテスト (`docs/editor.md#はみ出しの検証`)。

use std::collections::BTreeMap;

use ugoe_core::Vec2;
use ugoe_model::deformer::{DeformerKind, RotationKeyform, WarpKeyform};
use ugoe_model::{KeyformBinding, KeyformGrid, Model, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::KeyformTarget;
use crate::overflow::{
    DEFAULT_TOLERANCE, OverflowError, OverflowPointKind, overflows_at, uv_overflow,
    verify_overflows,
};
use crate::space::ParameterValues;

fn values(entries: &[(&str, f32)]) -> ParameterValues {
    entries.iter().map(|&(p, v)| (p.into(), v)).collect()
}

fn grid1<T>(parameter: &str, keys: &[f32], keyforms: Vec<T>) -> KeyformGrid<T> {
    KeyformGrid::new(
        vec![KeyformBinding {
            parameter: parameter.into(),
            keys: keys.to_vec(),
        }],
        keyforms,
    )
}

fn deformer_kind<'a>(model: &'a mut Model, id: &str) -> &'a mut DeformerKind {
    &mut model
        .deformers
        .iter_mut()
        .find(|d| d.id == id)
        .unwrap()
        .kind
}

/// `rotation_under_warp` の `Head` の原点を `Turn = 1` で UV `(0.5, -0.2)` (格子の上へ 0.2) にし、
/// `Cheek` の制御点を `Turn = 1` で 0.5 下げる (下の行が `v = 1.5` になる)。
fn turning_model() -> Model {
    let mut model = fixtures::rotation_under_warp();
    let DeformerKind::Rotation(head) = deformer_kind(&mut model, "Head") else {
        panic!("Head is a rotation deformer");
    };
    let rest = head.keyforms.keyforms[0];
    let raised = RotationKeyform {
        origin: Vec2::new(0.5, -0.2),
        ..rest
    };
    head.keyforms = grid1("Turn", &[0.0, 1.0], vec![rest, raised]);
    let DeformerKind::Warp(cheek) = deformer_kind(&mut model, "Cheek") else {
        panic!("Cheek is a warp");
    };
    let rest = cheek.keyforms.keyforms[0].clone();
    let lowered = WarpKeyform {
        points: rest
            .points
            .iter()
            .map(|p| *p + Vec2::new(0.0, 0.5))
            .collect(),
        ..rest.clone()
    };
    cheek.keyforms = grid1("Turn", &[0.0, 1.0], vec![rest, lowered]);
    model
}

#[test]
fn uv_overflow_is_the_distance_to_the_unit_square() {
    assert_eq!(uv_overflow(Vec2::new(0.5, 0.5)), 0.0);
    assert_eq!(uv_overflow(Vec2::new(0.0, 1.0)), 0.0);
    assert!((uv_overflow(Vec2::new(1.25, 0.5)) - 0.25).abs() < 1e-6);
    assert!((uv_overflow(Vec2::new(-0.3, 1.4)) - 0.5).abs() < 1e-6);
}

#[test]
fn finds_mesh_vertices_outside_the_parent_warp() {
    let found = overflows_at(
        &fixtures::warp_grid(),
        &ParameterValues::new(),
        DEFAULT_TOLERANCE,
    )
    .unwrap();
    // `Inner` は格子の内側、`Outer` は 4 頂点とも UV (-0.5..1.5) で外側。
    assert_eq!(found.len(), 1);
    let outer = &found[0];
    assert_eq!(outer.target, KeyformTarget::Mesh("Outer".into()));
    assert_eq!(outer.parent.as_str(), "Warp");
    assert_eq!(outer.kind, OverflowPointKind::Vertex);
    let indices: Vec<u32> = outer.points.iter().map(|p| p.index).collect();
    assert_eq!(indices, [0, 1, 2, 3]);
    for p in &outer.points {
        assert!((p.amount - 0.5f32.hypot(0.5)).abs() < 1e-5, "{p:?}");
    }
}

#[test]
fn elements_without_a_warp_parent_are_not_checked() {
    // 親なし・親が回転デフォーマの要素は UV 空間を持たない。
    let mut model = fixtures::rotation_chain();
    model.meshes[0].keyforms.keyforms[0].positions[0] = Vec2::new(-500.0, -500.0);
    let found = overflows_at(&model, &ParameterValues::new(), DEFAULT_TOLERANCE).unwrap();
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        verify_overflows(&model, DEFAULT_TOLERANCE)
            .unwrap()
            .elements,
        1
    );
}

#[test]
fn finds_rotation_origins_and_control_points_at_the_current_values() {
    let model = turning_model();
    assert!(
        overflows_at(&model, &ParameterValues::new(), DEFAULT_TOLERANCE)
            .unwrap()
            .is_empty()
    );
    // Turn = 0.5: 原点は v = 0.025、制御点の下の行は v = 1.25。
    let found = overflows_at(&model, &values(&[("Turn", 0.5)]), DEFAULT_TOLERANCE).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].target, KeyformTarget::Deformer("Cheek".into()));
    assert_eq!(found[0].kind, OverflowPointKind::ControlPoint);
    assert_eq!(found[0].points.len(), 2);
    assert_eq!(found[0].points[0].index, 2);
    assert!((found[0].points[0].amount - 0.25).abs() < 1e-5);

    let found = overflows_at(&model, &values(&[("Turn", 1.0)]), DEFAULT_TOLERANCE).unwrap();
    let head = found
        .iter()
        .find(|e| e.target == KeyformTarget::Deformer("Head".into()))
        .unwrap();
    assert_eq!(head.kind, OverflowPointKind::Origin);
    assert_eq!(head.points.len(), 1);
    assert!((head.points[0].uv - Vec2::new(0.5, -0.2)).length() < 1e-6);
    assert!((head.points[0].amount - 0.2).abs() < 1e-6);
}

#[test]
fn tolerance_ignores_points_just_outside() {
    let model = turning_model();
    // Turn = 0.01: 制御点の下の行は v = 1.005。
    let at = values(&[("Turn", 0.01)]);
    assert_eq!(
        overflows_at(&model, &at, DEFAULT_TOLERANCE).unwrap().len(),
        1
    );
    assert!(overflows_at(&model, &at, 0.01).unwrap().is_empty());
}

#[test]
fn verification_lists_every_overflowing_cell() {
    let report = verify_overflows(&turning_model(), DEFAULT_TOLERANCE).unwrap();
    // 親がワープの要素は Head・Cheek (デフォーマ) と Blush (メッシュ)。セルは Head と Cheek が 2 つ、
    // Blush は軸がなく 1 つ。
    assert_eq!(report.elements, 3);
    assert_eq!(report.cells, 5);
    assert!(report.truncated.is_empty());
    let summary: Vec<(String, BTreeMap<String, f32>, u32)> = report
        .findings
        .iter()
        .map(|f| {
            let cell = f.cell.iter().map(|(k, v)| (k.to_string(), *v)).collect();
            (f.target.to_string(), cell, f.count)
        })
        .collect();
    let turn1: BTreeMap<String, f32> = [("Turn".to_string(), 1.0)].into();
    assert_eq!(
        summary,
        [
            ("deformer `Head`".to_string(), turn1.clone(), 1),
            ("deformer `Cheek`".to_string(), turn1, 2),
        ]
    );
    let cheek = &report.findings[1];
    assert_eq!(cheek.kind, OverflowPointKind::ControlPoint);
    assert_eq!(cheek.worst_index, 2);
    assert!((cheek.max_amount - 0.5).abs() < 1e-5);
}

#[test]
fn verification_covers_the_combinations_of_axis_ends() {
    // 2 軸の格子で、両方の軸が端のときだけはみ出す (1 軸ずつ動かしただけでは見つからない)。
    let mut model = fixtures::blend_shapes();
    model
        .parameters
        .push(model.parameters[0].clone().with_id("Y"));
    let cheek = &mut model.meshes[1];
    assert_eq!(cheek.id.as_str(), "Cheek");
    let rest = cheek.keyforms.keyforms[0].clone();
    let shifted = |dx: f32| {
        let mut k = rest.clone();
        for p in &mut k.positions {
            p.x += dx;
        }
        k
    };
    let mut keyforms = Vec::new();
    for y in [-1.0f32, 1.0] {
        for x in [-1.0f32, 1.0] {
            keyforms.push(shifted(0.2 * x + 0.2 * y));
        }
    }
    cheek.keyforms = KeyformGrid::new(
        vec![
            KeyformBinding {
                parameter: "X".into(),
                keys: vec![-1.0, 1.0],
            },
            KeyformBinding {
                parameter: "Y".into(),
                keys: vec![-1.0, 1.0],
            },
        ],
        keyforms,
    );
    let report = verify_overflows(&model, DEFAULT_TOLERANCE).unwrap();
    let cells: Vec<Vec<(String, f32)>> = report
        .findings
        .iter()
        .map(|f| f.cell.iter().map(|(k, v)| (k.to_string(), *v)).collect())
        .collect();
    // 右端は x = 0.75 + 0.4 = 1.15、左端は x = 0.25 - 0.4 = -0.15。
    assert_eq!(
        cells,
        [
            vec![("X".to_string(), -1.0), ("Y".to_string(), -1.0)],
            vec![("X".to_string(), 1.0), ("Y".to_string(), 1.0)],
        ]
    );
    for f in &report.findings {
        assert_eq!(f.count, 2);
        assert!((f.max_amount - 0.15).abs() < 1e-5, "{f:?}");
    }
}

#[test]
fn blend_shapes_are_included_like_the_runtime() {
    let mut model = fixtures::blend_shapes();
    // `Cheek` (親 `Face`) にブレンドシェイプ格子 `Smile` を加え、`Smile = 1` で頂点 0 を左へ 0.5 動かす。
    let rest = model.meshes[1].keyforms.keyforms[0].clone();
    let mut smile = rest.clone();
    smile.positions[0].x -= 0.5;
    model.meshes[1].keyforms.blend_shapes = vec![grid1("Smile", &[0.0, 1.0], vec![rest, smile])];

    // 局所座標はランタイムの補間と一致する (許容誤差を負にするとすべての点が返る)。
    let at = values(&[("Smile", 0.5), ("Wide", 0.8)]);
    let all = overflows_at(&model, &at, -1.0).unwrap();
    let cheek = all
        .iter()
        .find(|e| e.target == KeyformTarget::Mesh("Cheek".into()))
        .unwrap();
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    for (id, value) in &at {
        let index = instance.parameters().index_of(id.as_str()).unwrap();
        instance.parameters_mut().set(index, *value);
    }
    let expected = instance.mesh_local_positions(1).unwrap();
    assert_eq!(cheek.points.len(), expected.len());
    for (p, q) in cheek.points.iter().zip(&expected) {
        assert!((p.uv - *q).length() < 1e-6, "{p:?} vs {q:?}");
    }

    let found = overflows_at(&model, &values(&[("Smile", 1.0)]), DEFAULT_TOLERANCE).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].points[0].index, 0);
    assert!((found[0].points[0].amount - 0.25).abs() < 1e-5);

    let report = verify_overflows(&model, DEFAULT_TOLERANCE).unwrap();
    assert_eq!(report.findings.len(), 1);
    let cell: Vec<(String, f32)> = report.findings[0]
        .cell
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
    assert_eq!(cell, [("Smile".to_string(), 1.0)]);
}

#[test]
fn too_many_combinations_check_only_the_axis_ends() {
    let mut model = fixtures::warp_grid();
    // `Inner` の格子に 3 キーの軸を 8 本 (3^8 = 6561 > MAX_COMBINATIONS) 付ける。
    let mut bindings = Vec::new();
    for i in 0..8 {
        let id = format!("P{i}");
        model
            .parameters
            .push(model.parameters[0].clone().with_id(&id));
        bindings.push(KeyformBinding {
            parameter: id.as_str().into(),
            keys: vec![-1.0, 0.0, 1.0],
        });
    }
    let rest = model.meshes[0].keyforms.keyforms[0].clone();
    model.meshes[0].keyforms = KeyformGrid::new(bindings, vec![rest; 6561]);
    let report = verify_overflows(&model, DEFAULT_TOLERANCE).unwrap();
    assert_eq!(report.truncated, [KeyformTarget::Mesh("Inner".into())]);
    // Inner は両端の 2^8 セル、Outer は 1 セル。
    assert_eq!(report.cells, 256 + 1);
}

#[test]
fn unknown_parameters_are_errors() {
    let mut model = fixtures::warp_grid();
    model.meshes[0].keyforms.bindings = vec![KeyformBinding {
        parameter: "Missing".into(),
        keys: vec![0.0],
    }];
    let error = verify_overflows(&model, DEFAULT_TOLERANCE).unwrap_err();
    assert!(
        matches!(error, OverflowError::UnknownParameter { .. }),
        "{error}"
    );
}

/// テスト用: ID と名前だけを変えたパラメータ。
trait WithId {
    fn with_id(self, id: &str) -> Self;
}

impl WithId for ugoe_model::Parameter {
    fn with_id(mut self, id: &str) -> Self {
        self.id = id.into();
        self.name = id.into();
        self
    }
}

#[test]
fn warp_control_points_match_the_runtime() {
    // 子のワープの制御点を親のワールド変換で写すと、ランタイムが評価したキャンバス座標の制御点になる。
    let model = turning_model();
    let at = values(&[("Turn", 0.3)]);
    let all = overflows_at(&model, &at, -1.0).unwrap();
    let cheek = all
        .iter()
        .find(|e| e.target == KeyformTarget::Deformer("Cheek".into()))
        .unwrap();
    let compiled = CompiledModel::compile(model.clone()).unwrap();
    let body = compiled.deformer_index("Body").unwrap();
    let index = compiled.deformer_index("Cheek").unwrap();
    let mut instance = ModelInstance::new(compiled);
    let turn = instance.parameters().index_of("Turn").unwrap();
    instance.parameters_mut().set(turn, 0.3);
    instance.update();
    let expected = instance.warp_world_points(index).unwrap();
    assert_eq!(cheek.points.len(), expected.len());
    for (p, q) in cheek.points.iter().zip(expected) {
        let world = instance.deformer_world(body).unwrap().point(p.uv);
        assert!((world - *q).length() < 1e-3, "{world:?} vs {q:?}");
    }
}

#[test]
fn smooth_axes_are_checked_between_keys() {
    // `Inner` を X のキー -1, 0, 1 で 0, -0.25, -0.25 だけ左へずらす。キーの上では左端が u = 0 (内側) だが、
    // 曲線補間では 0 と 1 の間で行き過ぎて格子の外に出る。
    let mut model = fixtures::warp_grid();
    model.meshes.truncate(1);
    let rest = model.meshes[0].keyforms.keyforms[0].clone();
    let shifted = |dx: f32| {
        let mut k = rest.clone();
        for p in &mut k.positions {
            p.x += dx;
        }
        k
    };
    let mut grid = grid1(
        "X",
        &[-1.0, 0.0, 1.0],
        vec![shifted(0.0), shifted(-0.25), shifted(-0.25)],
    );
    model.meshes[0].keyforms = grid.clone();
    let linear = verify_overflows(&model, DEFAULT_TOLERANCE).unwrap();
    assert!(linear.findings.is_empty(), "{linear:?}");

    grid.interpolation = ugoe_model::KeyformInterpolation::Smooth;
    model.meshes[0].keyforms = grid;
    let smooth = verify_overflows(&model, DEFAULT_TOLERANCE).unwrap();
    assert!(!smooth.findings.is_empty());
    for f in &smooth.findings {
        let x = f.cell.values().next().copied().unwrap();
        assert!(x > 0.0 && x < 1.0, "{f:?}");
        assert_eq!(f.count, 2);
    }
    // 今の値での検出も曲線補間の重みを使う。
    let at = values(&[(
        "X",
        smooth.findings[0].cell.values().next().copied().unwrap(),
    )]);
    assert_eq!(
        overflows_at(&model, &at, DEFAULT_TOLERANCE).unwrap().len(),
        1
    );
}
