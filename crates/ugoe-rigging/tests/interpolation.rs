//! 曲線補間の格子の推定 (`docs/spec/evaluation.md#曲線補間`)。
//!
//! 推定はセルの値だけを作り、補間の種類はセルの値に影響しない。曲線補間の格子でも線形の格子と
//! 同じセルを作り、補間の種類をそのまま残すことを確かめる。

use ugoe_model::{DeformerKind, KeyformInterpolation, Model, fixtures};
use ugoe_rigging::{
    InferScope, KeyformTarget, MirrorOptions, infer_additive_target, infer_depth_rotation,
    mirror_keyforms,
};

/// すべてのメッシュとデフォーマの格子を曲線補間にしたモデル。
fn smooth(mut model: Model) -> Model {
    for mesh in &mut model.meshes {
        mesh.keyforms.interpolation = KeyformInterpolation::Smooth;
    }
    for deformer in &mut model.deformers {
        match &mut deformer.kind {
            DeformerKind::Warp(w) => w.keyforms.interpolation = KeyformInterpolation::Smooth,
            DeformerKind::Rotation(r) => r.keyforms.interpolation = KeyformInterpolation::Smooth,
            _ => {}
        }
    }
    model
}

fn target(id: &str) -> KeyformTarget {
    KeyformTarget::Mesh(id.into())
}

fn run(model: &mut Model) {
    let face = KeyformTarget::Deformer("Face".into());
    infer_depth_rotation(model, &face, InferScope::Own).expect("depth rotation");
    infer_additive_target(model, &target("Nose"), InferScope::Own).expect("additive");
    let options = MirrorOptions::default();
    mirror_keyforms(model, &target("EyeL"), &target("EyeR"), &options).expect("mirror");
}

#[test]
fn inference_on_smooth_grids_matches_linear_grids() {
    let mut linear = fixtures::keyform_inference();
    let mut curved = smooth(linear.clone());
    run(&mut linear);
    run(&mut curved);
    // 補間の種類だけが違う。
    assert_eq!(smooth(linear), curved);
    assert!(
        curved
            .meshes
            .iter()
            .all(|m| m.keyforms.interpolation == KeyformInterpolation::Smooth)
    );
}
