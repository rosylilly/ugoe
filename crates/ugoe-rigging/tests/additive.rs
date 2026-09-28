//! 加算合成のテスト (`docs/spec/keyform-inference.md#加算合成`)。

use ugoe_core::{Rgb, Vec2};
use ugoe_model::deformer::{RotationKeyform, WarpKeyform};
use ugoe_model::mesh::{GlueKeyform, MeshKeyform};
use ugoe_model::part::PartKeyform;
use ugoe_model::{
    DeformerKind, KeyformBinding, KeyformGrid, KeyformOrigin, Model, Parameter, fixtures,
};
use ugoe_rigging::{InferError, KeyformTarget, infer_additive, infer_additive_target};

const EPS: f32 = 1e-4;

fn parameter(id: &str, default: f32) -> Parameter {
    Parameter {
        id: id.into(),
        name: id.into(),
        min: -100.0,
        max: 100.0,
        default,
        kind: Default::default(),
        mirror: None,
        group: None,
    }
}

/// パラメータ `A`〜`D` (既定 0) と `E` (既定 1) を持つモデル。
fn model() -> Model {
    let mut model = fixtures::keyform_inference();
    model.parameters = ["A", "B", "C", "D"]
        .iter()
        .map(|id| parameter(id, 0.0))
        .chain([parameter("E", 1.0)])
        .collect();
    model
}

fn binding(parameter: &str, keys: &[f32]) -> KeyformBinding {
    KeyformBinding {
        parameter: parameter.into(),
        keys: keys.to_vec(),
    }
}

fn rot(angle: f32) -> RotationKeyform {
    RotationKeyform {
        origin: Vec2::ZERO,
        angle,
        scale: 1.0,
        reflect_x: false,
        reflect_y: false,
        opacity: 1.0,
    }
}

/// 回転の角度を 1 つの数値として扱う格子。`authored` の添字以外は `derived` (値は 999)。
fn angle_grid(
    bindings: Vec<KeyformBinding>,
    authored: &[(usize, f32)],
) -> KeyformGrid<RotationKeyform> {
    let len = bindings.iter().map(|b| b.keys.len()).product();
    let mut grid = KeyformGrid::new(bindings, vec![rot(999.0); len]);
    grid.origins = vec![KeyformOrigin::Derived; len];
    for &(i, angle) in authored {
        grid.keyforms[i] = rot(angle);
        grid.origins[i] = KeyformOrigin::Authored;
    }
    grid
}

fn angles(grid: &KeyformGrid<RotationKeyform>) -> Vec<f32> {
    grid.keyforms.iter().map(|k| k.angle).collect()
}

fn assert_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() < EPS, "{actual:?} vs {expected:?}");
    }
}

/// 2 値 (0, 1) の軸 `n` 本の格子の線形インデックス。`on` は中立でない軸の一覧。
fn corner(on: &[usize]) -> usize {
    on.iter().map(|k| 1 << k).sum()
}

#[test]
fn one_axis_interpolates_between_authored_cells() {
    let model = model();
    // キー -2 -1 0 1 2 (中立は 0 = 添字 2)。-2 と 0 と 2 が authored。
    let mut grid = angle_grid(
        vec![binding("A", &[-2.0, -1.0, 0.0, 1.0, 2.0])],
        &[(0, -40.0), (2, 0.0), (4, 20.0)],
    );
    let report = infer_additive(&model, &mut grid).unwrap();
    assert_eq!(report.written, vec![1, 3]);
    assert_close(&angles(&grid), &[-40.0, -20.0, 0.0, 10.0, 20.0]);
}

#[test]
fn one_axis_interpolates_with_uneven_keys() {
    let model = model();
    // 基準 (0) と 4 の間の 1 は 1/4 の位置。
    let mut grid = angle_grid(vec![binding("A", &[0.0, 1.0, 4.0])], &[(0, 0.0), (2, 40.0)]);
    infer_additive(&model, &mut grid).unwrap();
    assert_close(&angles(&grid), &[0.0, 10.0, 40.0]);
}

#[test]
fn one_axis_extrapolates_from_the_nearest_two_authored_cells() {
    let model = model();
    // キー 0 1 2 4。0 (基準) 1 2 が authored。4 は最も近い 2 と 1 から外挿する。
    let mut grid = angle_grid(
        vec![binding("A", &[0.0, 1.0, 2.0, 4.0])],
        &[(0, 0.0), (1, 10.0), (2, 30.0)],
    );
    infer_additive(&model, &mut grid).unwrap();
    assert_close(&angles(&grid), &[0.0, 10.0, 30.0, 70.0]);

    // 片側に基準のセルと 1 つだけ: 基準と組んで外挿する。
    let mut grid = angle_grid(
        vec![binding("A", &[-2.0, -1.0, 0.0, 1.0])],
        &[(2, 5.0), (3, 15.0)],
    );
    infer_additive(&model, &mut grid).unwrap();
    assert_close(&angles(&grid), &[-15.0, -5.0, 5.0, 15.0]);
}

#[test]
fn one_axis_with_only_the_base_copies_the_base() {
    let model = model();
    let mut grid = angle_grid(vec![binding("A", &[-1.0, 0.0, 1.0])], &[(1, 7.0)]);
    infer_additive(&model, &mut grid).unwrap();
    assert_close(&angles(&grid), &[7.0, 7.0, 7.0]);
}

#[test]
fn neutral_key_follows_the_parameter_default() {
    let model = model();
    // E の既定は 1 なので、添字 1 が基準のセル。
    let mut grid = angle_grid(
        vec![binding("E", &[0.0, 1.0, 2.0])],
        &[(1, 10.0), (2, 30.0)],
    );
    infer_additive(&model, &mut grid).unwrap();
    assert_close(&angles(&grid), &[-10.0, 10.0, 30.0]);
}

#[test]
fn two_axis_diagonal_adds_the_one_axis_changes() {
    let model = model();
    let keys = [-1.0, 0.0, 1.0];
    let offset = |x: f32, y: f32, angle: f32| RotationKeyform {
        origin: Vec2::new(100.0 + x, 50.0 + y),
        angle,
        ..rot(0.0)
    };
    let mut grid = KeyformGrid::new(
        vec![binding("A", &keys), binding("B", &keys)],
        vec![offset(0.0, 0.0, 0.0); 9],
    );
    grid.origins = vec![KeyformOrigin::Derived; 9];
    // 格子座標 (a, b) の添字は a + 3 * b。基準は (1, 1) = 4。
    for (i, kf) in [
        (4, offset(0.0, 0.0, 0.0)),
        (3, offset(-10.0, 0.0, -30.0)),
        (5, offset(10.0, 0.0, 30.0)),
        (1, offset(0.0, -5.0, 3.0)),
        (7, offset(0.0, 5.0, -3.0)),
    ] {
        grid.keyforms[i] = kf;
        grid.origins[i] = KeyformOrigin::Authored;
    }
    let report = infer_additive(&model, &mut grid).unwrap();
    assert_eq!(report.written, vec![0, 2, 6, 8]);
    for (i, expected) in [
        (0, offset(-10.0, -5.0, -27.0)),
        (2, offset(10.0, -5.0, 33.0)),
        (6, offset(-10.0, 5.0, -33.0)),
        (8, offset(10.0, 5.0, 27.0)),
    ] {
        let kf = &grid.keyforms[i];
        assert_close(
            &[kf.origin.x, kf.origin.y, kf.angle],
            &[expected.origin.x, expected.origin.y, expected.angle],
        );
    }
}

#[test]
fn two_axis_diagonal_uses_inferred_one_axis_cells() {
    let model = model();
    // A=2 は A=1 から外挿した 1 軸セル。(2, 1) は その Δ と B の Δ の和。
    let mut grid = angle_grid(
        vec![binding("A", &[0.0, 1.0, 2.0]), binding("B", &[0.0, 1.0])],
        &[(0, 0.0), (1, 10.0), (3, 100.0)],
    );
    infer_additive(&model, &mut grid).unwrap();
    assert_close(&angles(&grid), &[0.0, 10.0, 20.0, 100.0, 110.0, 120.0]);
}

#[test]
fn authored_two_axis_correction_propagates_into_three_axis_cells() {
    let model = model();
    let keys = [0.0, 1.0];
    let bindings = || {
        vec![
            binding("A", &keys),
            binding("B", &keys),
            binding("C", &keys),
        ]
    };
    // Δ_A = 1, Δ_B = 10, Δ_C = 100。AB の斜めを手で 5 だけ直す (R(AB) = 5)。
    let mut grid = angle_grid(
        bindings(),
        &[
            (corner(&[]), 1000.0),
            (corner(&[0]), 1001.0),
            (corner(&[1]), 1010.0),
            (corner(&[2]), 1100.0),
            (corner(&[0, 1]), 1016.0),
        ],
    );
    let report = infer_additive(&model, &mut grid).unwrap();
    assert_eq!(
        report.written,
        vec![corner(&[0, 2]), corner(&[1, 2]), corner(&[0, 1, 2])]
    );
    let a = angles(&grid);
    assert_close(&[a[corner(&[0, 2])]], &[1101.0]);
    assert_close(&[a[corner(&[1, 2])]], &[1110.0]);
    // B + Δ_A + Δ_B + Δ_C + R(AB)
    assert_close(&[a[corner(&[0, 1, 2])]], &[1116.0]);
}

#[test]
fn interaction_terms_are_defined_recursively() {
    let model = model();
    let keys = [0.0, 1.0];
    let bindings = ["A", "B", "C", "D"]
        .iter()
        .map(|p| binding(p, &keys))
        .collect();
    // Δ = 1, 10, 100, 1000。R(AB) = 5、R(AC) = 7。
    // ABC は 1111 + 5 + 7 + 3 で authored なので R(ABC) = 3。
    let mut grid = angle_grid(
        bindings,
        &[
            (corner(&[]), 0.0),
            (corner(&[0]), 1.0),
            (corner(&[1]), 10.0),
            (corner(&[2]), 100.0),
            (corner(&[3]), 1000.0),
            (corner(&[0, 1]), 16.0),
            (corner(&[0, 2]), 108.0),
            (corner(&[0, 1, 2]), 126.0),
        ],
    );
    infer_additive(&model, &mut grid).unwrap();
    let a = angles(&grid);
    // BC は authored でないので R(BC) = 0。
    assert_close(&[a[corner(&[1, 2])]], &[110.0]);
    // ABD = B + Δ_A + Δ_B + Δ_D + R(AB)
    assert_close(&[a[corner(&[0, 1, 3])]], &[1016.0]);
    // ACD = ... + R(AC)
    assert_close(&[a[corner(&[0, 2, 3])]], &[1108.0]);
    // BCD には交互作用の項がない
    assert_close(&[a[corner(&[1, 2, 3])]], &[1110.0]);
    // ABCD = B + ΣΔ + R(AB) + R(AC) + R(ABC) = ABC + Δ_D
    assert_close(&[a[corner(&[0, 1, 2, 3])]], &[1126.0]);
}

fn mesh_kf(offset: f32, opacity: f32, color: f32, mask_strength: f32) -> MeshKeyform {
    MeshKeyform {
        positions: vec![Vec2::new(offset, 0.0), Vec2::new(offset, 10.0)],
        opacity,
        draw_order: 500.0 + offset,
        multiply_color: Rgb {
            r: color,
            g: 1.0,
            b: 1.0,
        },
        screen_color: Rgb {
            r: 1.0 - color,
            g: 0.0,
            b: 0.0,
        },
        mask_strength,
    }
}

#[test]
fn mesh_fields_are_added_and_clamped() {
    let model = model();
    let keys = [0.0, 1.0];
    let mut grid = KeyformGrid::new(
        vec![binding("A", &keys), binding("B", &keys)],
        vec![
            mesh_kf(0.0, 0.6, 0.5, 0.5),
            mesh_kf(1.0, 1.0, 1.0, 1.0),
            mesh_kf(2.0, 1.0, 0.9, 0.0),
            mesh_kf(0.0, 0.0, 0.0, 0.0),
        ],
    );
    grid.origins = vec![
        KeyformOrigin::Authored,
        KeyformOrigin::Authored,
        KeyformOrigin::Authored,
        KeyformOrigin::Derived,
    ];
    infer_additive(&model, &mut grid).unwrap();
    let kf = &grid.keyforms[3];
    // 位置と描画順はクランプしない。
    assert_close(
        &[kf.positions[0].x, kf.positions[1].y, kf.draw_order],
        &[3.0, 10.0, 503.0],
    );
    // 0.6 + 0.4 + 0.4 = 1.4 -> 1
    assert_close(&[kf.opacity], &[1.0]);
    // 乗算色 r: 0.5 + 0.5 + 0.4 = 1.4 -> 1、スクリーン色 r: 0.5 - 0.5 - 0.4 -> 0
    assert_close(&[kf.multiply_color.r, kf.screen_color.r], &[1.0, 0.0]);
    // 0.5 + 0.5 - 0.5 = 0.5
    assert_close(&[kf.mask_strength], &[0.5]);

    grid.keyforms[1].mask_strength = 0.0;
    infer_additive(&model, &mut grid).unwrap();
    // 0.5 - 0.5 - 0.5 -> 0
    assert_close(&[grid.keyforms[3].mask_strength], &[0.0]);
}

#[test]
fn rotation_scale_is_clamped_and_booleans_come_from_the_base() {
    let model = model();
    let keys = [0.0, 1.0];
    let kf = |scale: f32, reflect: bool| RotationKeyform {
        scale,
        reflect_x: reflect,
        reflect_y: !reflect,
        ..rot(0.0)
    };
    let mut grid = KeyformGrid::new(
        vec![binding("A", &keys), binding("B", &keys)],
        vec![kf(1.0, false), kf(0.2, true), kf(0.3, true), kf(9.0, true)],
    );
    grid.origins = vec![
        KeyformOrigin::Authored,
        KeyformOrigin::Authored,
        KeyformOrigin::Authored,
        KeyformOrigin::Derived,
    ];
    infer_additive(&model, &mut grid).unwrap();
    // 1 - 0.8 - 0.7 -> 0
    assert_eq!(grid.keyforms[3].scale, 0.0);
    assert!(!grid.keyforms[3].reflect_x);
    assert!(grid.keyforms[3].reflect_y);
}

#[test]
fn part_and_glue_values_are_clamped() {
    let model = model();
    let bindings = || vec![binding("A", &[0.0, 1.0, 2.0])];
    let mut part = KeyformGrid::new(
        bindings(),
        vec![
            PartKeyform { opacity: 0.2 },
            PartKeyform { opacity: 0.7 },
            PartKeyform { opacity: 0.0 },
        ],
    );
    part.origins = vec![
        KeyformOrigin::Authored,
        KeyformOrigin::Authored,
        KeyformOrigin::Derived,
    ];
    infer_additive(&model, &mut part).unwrap();
    // 0.2 + 2 * 0.5 = 1.2 -> 1
    assert_eq!(part.keyforms[2].opacity, 1.0);

    let mut glue = KeyformGrid::new(
        bindings(),
        vec![
            GlueKeyform { intensity: 0.5 },
            GlueKeyform { intensity: 0.2 },
            GlueKeyform { intensity: 1.0 },
        ],
    );
    glue.origins = part.origins.clone();
    infer_additive(&model, &mut glue).unwrap();
    // 0.5 - 2 * 0.3 = -0.1 -> 0
    assert_eq!(glue.keyforms[2].intensity, 0.0);
}

#[test]
fn authored_cells_are_never_overwritten() {
    let model = model();
    let keys = [-1.0, 0.0, 1.0];
    let authored = [
        (4, 0.0),
        (3, -3.0),
        (5, 3.0),
        (1, 1.0),
        (7, -1.0),
        (8, 42.0),
    ];
    let mut grid = angle_grid(vec![binding("A", &keys), binding("B", &keys)], &authored);
    let before = grid.clone();
    let report = infer_additive(&model, &mut grid).unwrap();
    assert_eq!(report.written, vec![0, 2, 6]);
    assert_eq!(grid.origins, before.origins);
    for (i, _) in authored {
        assert_eq!(grid.keyforms[i], before.keyforms[i], "cell {i}");
    }
    // 何度実行しても同じ結果 (決定的)。
    let once = grid.clone();
    infer_additive(&model, &mut grid).unwrap();
    assert_eq!(grid, once);
}

#[test]
fn grid_without_origins_is_all_authored() {
    let model = model();
    let mut grid = KeyformGrid::new(vec![binding("A", &[0.0, 1.0])], vec![rot(0.0), rot(123.0)]);
    let before = grid.clone();
    let report = infer_additive(&model, &mut grid).unwrap();
    assert!(report.written.is_empty());
    assert_eq!(grid, before);
    assert!(grid.origins.is_empty());
}

#[test]
fn axis_without_neutral_key_is_an_error() {
    let model = model();
    let mut grid = angle_grid(
        vec![binding("A", &[0.0, 1.0]), binding("B", &[-1.0, 1.0])],
        &[(0, 0.0)],
    );
    let before = grid.clone();
    let err = infer_additive(&model, &mut grid).unwrap_err();
    assert_eq!(
        err,
        InferError::MissingNeutralKey {
            parameter: "B".into(),
            default: 0.0
        }
    );
    assert!(err.to_string().contains('B'), "{err}");
    assert_eq!(grid, before);
}

#[test]
fn unknown_parameter_is_an_error() {
    let model = model();
    let mut grid = angle_grid(vec![binding("Nope", &[0.0, 1.0])], &[(0, 0.0)]);
    assert_eq!(
        infer_additive(&model, &mut grid),
        Err(InferError::UnknownParameter {
            parameter: "Nope".into()
        })
    );
}

#[test]
fn mismatched_vertex_counts_are_an_error() {
    let model = model();
    let mut short = mesh_kf(1.0, 1.0, 1.0, 1.0);
    short.positions.pop();
    let mut grid = KeyformGrid::new(
        vec![binding("A", &[0.0, 1.0, 2.0])],
        vec![
            mesh_kf(0.0, 1.0, 1.0, 1.0),
            short,
            mesh_kf(0.0, 1.0, 1.0, 1.0),
        ],
    );
    grid.origins = vec![
        KeyformOrigin::Authored,
        KeyformOrigin::Authored,
        KeyformOrigin::Derived,
    ];
    let before = grid.clone();
    assert_eq!(
        infer_additive(&model, &mut grid),
        Err(InferError::ShapeMismatch {
            cell: 1,
            expected: 2,
            found: 1
        })
    );
    assert_eq!(grid, before);
}

#[test]
fn derived_base_and_malformed_grids_are_errors() {
    let model = model();
    let mut grid = angle_grid(vec![binding("A", &[0.0, 1.0])], &[(1, 0.0)]);
    assert_eq!(
        infer_additive(&model, &mut grid),
        Err(InferError::BaseNotAuthored { cell: 0 })
    );

    let mut grid = angle_grid(vec![binding("A", &[0.0, 1.0])], &[(0, 0.0)]);
    grid.keyforms.pop();
    assert_eq!(
        infer_additive(&model, &mut grid),
        Err(InferError::GridSizeMismatch {
            expected: 2,
            found: 1
        })
    );

    let mut grid = angle_grid(vec![binding("A", &[0.0, 1.0])], &[(0, 0.0)]);
    grid.origins.pop();
    assert_eq!(
        infer_additive(&model, &mut grid),
        Err(InferError::OriginsLengthMismatch {
            expected: 2,
            found: 1
        })
    );
}

fn face(model: &Model) -> &KeyformGrid<WarpKeyform> {
    let deformer = model.deformers.iter().find(|d| d.id == "Face".into());
    match deformer.map(|d| &d.kind) {
        Some(DeformerKind::Warp(w)) => &w.keyforms,
        _ => panic!("Face is not a warp deformer"),
    }
}

fn face_mut(model: &mut Model) -> &mut KeyformGrid<WarpKeyform> {
    let deformer = model.deformers.iter_mut().find(|d| d.id == "Face".into());
    match deformer.map(|d| &mut d.kind) {
        Some(DeformerKind::Warp(w)) => &mut w.keyforms,
        _ => panic!("Face is not a warp deformer"),
    }
}

#[test]
fn fixture_face_warp_is_inferred_by_target() {
    let mut model = fixtures::keyform_inference();
    let shift = |kf: &WarpKeyform, d: Vec2| WarpKeyform {
        points: kf.points.iter().map(|&p| p + d).collect(),
        ..kf.clone()
    };
    // AngleX=30 (添字 5) を右へ 10、AngleY=30 (添字 7) を下へ 10 動かして authored にする。
    let base = face(&model).keyforms[4].clone();
    let grid = face_mut(&mut model);
    grid.keyforms[5] = shift(&base, Vec2::new(10.0, 0.0));
    grid.keyforms[7] = shift(&base, Vec2::new(0.0, 10.0));
    grid.origins[5] = KeyformOrigin::Authored;
    grid.origins[7] = KeyformOrigin::Authored;

    let target = KeyformTarget::Deformer("Face".into());
    let report = infer_additive_target(&mut model, &target).unwrap();
    assert_eq!(report.written, vec![0, 1, 2, 3, 6, 8]);
    let grid = face(&model);
    // 添字 a + 3 * b の期待する移動量。-30 側は基準と +30 から外挿する。
    for (i, d) in [
        (0, Vec2::new(-10.0, -10.0)),
        (1, Vec2::new(0.0, -10.0)),
        (2, Vec2::new(10.0, -10.0)),
        (3, Vec2::new(-10.0, 0.0)),
        (6, Vec2::new(-10.0, 10.0)),
        (8, Vec2::new(10.0, 10.0)),
    ] {
        let expected = shift(&base, d);
        for (p, e) in grid.keyforms[i].points.iter().zip(&expected.points) {
            assert!((*p - *e).length() < EPS, "cell {i}: {p} vs {e}");
        }
        assert_eq!(grid.origins[i], KeyformOrigin::Derived);
    }
}

#[test]
fn targets_resolve_every_element_kind() {
    let mut model = fixtures::keyform_inference();
    for target in [
        KeyformTarget::Mesh("EyeL".into()),
        KeyformTarget::Part("Root".into()),
        KeyformTarget::Deformer("Face".into()),
    ] {
        assert!(
            infer_additive_target(&mut model, &target).is_ok(),
            "{target:?}"
        );
    }
    for target in [
        KeyformTarget::Mesh("Nope".into()),
        KeyformTarget::Part("Nope".into()),
        KeyformTarget::Deformer("Nope".into()),
        KeyformTarget::Glue("Nope".into()),
    ] {
        assert_eq!(
            infer_additive_target(&mut model, &target),
            Err(InferError::UnknownTarget(target.clone()))
        );
    }

    let mut model = fixtures::glued_quads();
    let glue = model.glues[0].id.clone();
    assert!(infer_additive_target(&mut model, &KeyformTarget::Glue(glue)).is_ok());
}
