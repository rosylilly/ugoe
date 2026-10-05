//! 深度の編集 (`SetDepth`・`ApplyDepthPreset`) とプリセットの関数のテスト。

use ugoe_core::Vec2;
use ugoe_model::deformer::{DeformerKind, WarpDeformer};
use ugoe_model::{KeyformOrigin, Model, fixtures};

use crate::depth::{dome_depth, points_depth, uniform_depth};
use crate::{ChildRef, Command, DepthPreset, Document, EditError};

fn face(model: &Model) -> &WarpDeformer {
    match &model.deformers[0].kind {
        DeformerKind::Warp(w) => w,
        _ => panic!("Face is a warp"),
    }
}

fn mesh_depth(model: &Model, id: &str) -> Option<Vec<f32>> {
    model
        .meshes
        .iter()
        .find(|m| m.id == id)
        .unwrap()
        .depth
        .clone()
}

fn set_depth(target: ChildRef, depth: Option<Vec<f32>>) -> Command {
    Command::SetDepth { target, depth }
}

fn preset(target: ChildRef, preset: DepthPreset) -> Command {
    Command::ApplyDepthPreset { target, preset }
}

fn assert_close(a: &[f32], b: &[f32]) {
    assert_eq!(a.len(), b.len(), "{a:?} vs {b:?}");
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 1e-4, "{a:?} vs {b:?}");
    }
}

// ---- 純粋な関数 ----

#[test]
fn uniform_depth_repeats_the_value() {
    assert_eq!(uniform_depth(3, 2.5), vec![2.5; 3]);
    assert_eq!(uniform_depth(0, 1.0), Vec::<f32>::new());
}

#[test]
fn dome_depth_peaks_at_the_center_and_is_zero_on_the_ellipse() {
    // 3×3 の格子 (x: 0..40, y: 0..20)。外接矩形の中心は (20, 10)、半径は (20, 10)。
    let points: Vec<Vec2> = (0..3)
        .flat_map(|r| (0..3).map(move |c| Vec2::new(20.0 * c as f32, 10.0 * r as f32)))
        .collect();
    let depth = dome_depth(&points, 12.0);
    // 中心は peak、辺の中点は楕円の上 (0)、角は楕円の外 (0)。
    assert_close(&depth, &[0.0, 0.0, 0.0, 0.0, 12.0, 0.0, 0.0, 0.0, 0.0]);

    // 楕円の内側の点: ρ² = 0.25 → peak · sqrt(0.75)。横と縦で半径が違っても正規化される。
    let inner = dome_depth(
        &[
            Vec2::new(0.0, 0.0),
            Vec2::new(40.0, 20.0),
            Vec2::new(30.0, 10.0),
            Vec2::new(20.0, 15.0),
        ],
        8.0,
    );
    let expected = 8.0 * 0.75f32.sqrt();
    assert_close(&inner, &[0.0, 0.0, expected, expected]);

    // 負の peak はくぼみ。点がないなら空、すべて同じ点なら peak。
    assert_close(&dome_depth(&points, -4.0)[4..5], &[-4.0]);
    assert!(dome_depth(&[], 1.0).is_empty());
    assert_close(&dome_depth(&[Vec2::new(3.0, 3.0); 2], 5.0), &[5.0, 5.0]);
    // 幅が 0 の向きは数えない (縦に並んだ点)。
    let column = [0.0, 5.0, 10.0].map(|y| Vec2::new(1.0, y));
    assert_close(&dome_depth(&column, 2.0), &[0.0, 2.0, 0.0]);
}

#[test]
fn points_depth_sets_only_the_given_points() {
    assert_eq!(
        points_depth(Some(&[1.0, 2.0, 3.0]), 3, &[0, 2], 9.0),
        Ok(vec![9.0, 2.0, 9.0])
    );
    // 深度がない (または長さが合わない) なら 0 から始める。
    assert_eq!(points_depth(None, 3, &[1], 4.0), Ok(vec![0.0, 4.0, 0.0]));
    assert_eq!(points_depth(Some(&[1.0]), 2, &[1], 4.0), Ok(vec![0.0, 4.0]));
    assert_eq!(points_depth(None, 3, &[1, 3], 4.0), Err(3));
}

// ---- SetDepth ----

/// 深度からの回転 (yaw だけ) の式: `x' = cos(yaw)(x − ox) + sin(yaw)(d − od) + ox`、y はそのまま。
fn turn(p: Vec2, d: f32, yaw: f32) -> Vec2 {
    let (s, c) = yaw.to_radians().sin_cos();
    Vec2::new(c * (p.x - 128.0) + s * d + 128.0, p.y)
}

#[test]
fn set_depth_rederives_depth_rotation_cells() {
    // `Face` は深度からの回転の格子 (中央以外は `derived`)。新しい深度で作り直される。
    let model = fixtures::keyform_inference();
    let base = face(&model).keyforms.keyforms[4].points.clone();
    let mut doc = Document::new(model);
    let depth = vec![4.0, 8.0, 20.0, -6.0];
    doc.apply(set_depth(
        ChildRef::Deformer("Face".into()),
        Some(depth.clone()),
    ))
    .unwrap();
    let warp = face(doc.model());
    assert_eq!(warp.depth.as_deref(), Some(&depth[..]));
    // AngleX = 30, AngleY = 0 はインデックス 5 (軸 0 が最も速い)。
    assert_eq!(warp.keyforms.origin(5), KeyformOrigin::DepthRotation);
    for (i, p) in warp.keyforms.keyforms[5].points.iter().enumerate() {
        let expected = turn(base[i], depth[i], 30.0);
        assert!(
            (*p - expected).length() < 1e-3,
            "point {i}: {p} vs {expected}"
        );
    }
    assert_eq!(warp.keyforms.keyforms[4].points, base);

    // 深度からの回転のセルが残っている格子の深度は外せない (作り直しが失敗して拒否される)。
    let before = doc.model().clone();
    assert!(matches!(
        doc.apply(set_depth(ChildRef::Deformer("Face".into()), None)),
        Err(EditError::Inference { .. })
    ));
    assert_eq!(doc.model(), &before);
}

#[test]
fn set_depth_on_meshes_and_undo() {
    let mut doc = Document::new(fixtures::keyform_inference());
    doc.apply(set_depth(
        ChildRef::Mesh("Nose".into()),
        Some(vec![1.0, 2.0, 3.0, 4.0]),
    ))
    .unwrap();
    assert_eq!(
        mesh_depth(doc.model(), "Nose"),
        Some(vec![1.0, 2.0, 3.0, 4.0])
    );
    doc.apply(set_depth(ChildRef::Mesh("EyeL".into()), None))
        .unwrap();
    assert_eq!(mesh_depth(doc.model(), "EyeL"), None);
    assert!(doc.undo());
    assert_eq!(mesh_depth(doc.model(), "EyeL"), Some(vec![8.0; 4]));
    assert!(doc.undo());
    assert_eq!(mesh_depth(doc.model(), "Nose"), None);
}

#[test]
fn set_depth_rejects_invalid_requests() {
    let mut doc = Document::new(fixtures::keyform_inference());
    let before = doc.model().clone();
    assert_eq!(
        doc.apply(set_depth(ChildRef::Mesh("Nose".into()), Some(vec![1.0; 3]))),
        Err(EditError::DepthLength {
            target: "mesh `Nose`".into(),
            expected: 4,
            actual: 3,
        })
    );
    assert!(matches!(
        doc.apply(set_depth(
            ChildRef::Deformer("Face".into()),
            Some(vec![0.0, f32::NAN, 0.0, 0.0])
        )),
        Err(EditError::NonFiniteDepth { .. })
    ));
    assert!(matches!(
        doc.apply(set_depth(ChildRef::Mesh("Missing".into()), None)),
        Err(EditError::NotFound { kind: "mesh", .. })
    ));
    assert!(matches!(
        doc.apply(preset(
            ChildRef::Mesh("Nose".into()),
            DepthPreset::Uniform {
                value: f32::INFINITY
            }
        )),
        Err(EditError::NonFiniteDepth { .. })
    ));
    assert_eq!(
        doc.apply(preset(
            ChildRef::Mesh("Nose".into()),
            DepthPreset::Points {
                indices: vec![0, 4],
                value: 1.0
            }
        )),
        Err(EditError::DepthPointOutOfRange {
            target: "mesh `Nose`".into(),
            index: 4,
            points: 4,
        })
    );
    assert_eq!(doc.model(), &before);

    // 回転デフォーマは深度を持たない。
    let mut doc = Document::new(fixtures::rotation_chain());
    assert!(matches!(
        doc.apply(set_depth(ChildRef::Deformer("Arm".into()), None)),
        Err(EditError::DeformerKindMismatch {
            expected: "warp",
            ..
        })
    ));
}

// ---- ApplyDepthPreset ----

#[test]
fn presets_build_the_depth() {
    let mut doc = Document::new(fixtures::keyform_inference());
    doc.apply(preset(
        ChildRef::Mesh("Nose".into()),
        DepthPreset::Uniform { value: 3.0 },
    ))
    .unwrap();
    assert_eq!(mesh_depth(doc.model(), "Nose"), Some(vec![3.0; 4]));

    doc.apply(preset(
        ChildRef::Mesh("EyeL".into()),
        DepthPreset::Points {
            indices: vec![1, 2],
            value: -2.0,
        },
    ))
    .unwrap();
    assert_eq!(
        mesh_depth(doc.model(), "EyeL"),
        Some(vec![8.0, -2.0, -2.0, 8.0])
    );

    // ドーム: 鼻の中心に頂点を足すと、その頂点だけが peak 近くになり、四隅は 0。
    // 鼻は UV (0.45, 0.5)〜(0.55, 0.7) で、顔 (64〜192px) の中ではキャンバス (128, 140.8) が中心。
    doc.apply(Command::AddMeshVertex {
        id: "Nose".into(),
        position: Vec2::new(128.0, 140.8),
    })
    .unwrap();
    doc.apply(preset(
        ChildRef::Mesh("Nose".into()),
        DepthPreset::Dome { peak: 10.0 },
    ))
    .unwrap();
    let depth = mesh_depth(doc.model(), "Nose").unwrap();
    assert_close(&depth[..4], &[0.0; 4]);
    assert!((depth[4] - 10.0).abs() < 1e-2, "{depth:?}");
}

#[test]
fn dome_preset_on_a_depth_rotation_warp_rederives_its_cells() {
    // 1×1 のワープの制御点はすべて外接矩形の角なので、ドームは 0 (平らな板の回転) になる。
    let mut doc = Document::new(fixtures::keyform_inference());
    doc.apply(preset(
        ChildRef::Deformer("Face".into()),
        DepthPreset::Dome { peak: 30.0 },
    ))
    .unwrap();
    let warp = face(doc.model());
    assert_eq!(warp.depth, Some(vec![0.0; 4]));
    let base = &warp.keyforms.keyforms[4].points;
    for (i, p) in warp.keyforms.keyforms[5].points.iter().enumerate() {
        assert!((*p - turn(base[i], 0.0, 30.0)).length() < 1e-3);
    }
}
