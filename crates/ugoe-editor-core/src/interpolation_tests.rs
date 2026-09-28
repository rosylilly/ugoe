//! キーフォームの補間の種類の編集 (`SetKeyformInterpolation`) と、曲線補間の格子へのキーの追加のテスト
//! (`docs/spec/evaluation.md#曲線補間`)。

use ugoe_core::Vec2;
use ugoe_model::{KeyformInterpolation, Model, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::{Command, Document, EditError, KeyformTarget};

use KeyformInterpolation::{Linear, Smooth};

fn set(
    target: KeyformTarget,
    blend_shape: Option<&str>,
    interpolation: KeyformInterpolation,
) -> Command {
    Command::SetKeyformInterpolation {
        target,
        blend_shape: blend_shape.map(Into::into),
        interpolation,
    }
}

fn mesh(id: &str) -> KeyformTarget {
    KeyformTarget::Mesh(id.into())
}

/// `AngleX = angle` での `Cheek` の頂点・不透明度・描画順。
fn cheek(model: &Model, angle: f32) -> (Vec<Vec2>, f32, f32) {
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    let index = instance.parameters().index_of("AngleX").unwrap();
    instance.parameters_mut().set(index, angle);
    instance.update();
    let d = &instance.drawables()[0];
    (d.positions.clone(), d.opacity, d.draw_order)
}

#[test]
fn set_interpolation_on_a_grid_and_undo() {
    let mut doc = Document::new(fixtures::single_quad());
    doc.apply(set(mesh("Quad"), None, Smooth)).unwrap();
    assert_eq!(doc.model().meshes[0].keyforms.interpolation, Smooth);
    assert!(doc.undo());
    assert_eq!(doc.model().meshes[0].keyforms.interpolation, Linear);
    assert!(doc.redo());
    assert_eq!(doc.model().meshes[0].keyforms.interpolation, Smooth);
}

#[test]
fn set_interpolation_on_a_blend_shape_grid() {
    let mut doc = Document::new(fixtures::blend_shapes());
    doc.apply(set(mesh("Mouth"), Some("Wide"), Smooth)).unwrap();
    let grid = &doc.model().meshes[0].keyforms;
    assert_eq!(grid.interpolation, Linear);
    assert_eq!(grid.blend_shapes[0].interpolation, Linear);
    assert_eq!(grid.blend_shapes[1].interpolation, Smooth);
    let error = doc
        .apply(set(mesh("Mouth"), Some("X"), Smooth))
        .unwrap_err();
    assert!(
        matches!(
            error,
            EditError::NotFound {
                kind: "blend shape grid",
                ..
            }
        ),
        "{error:?}"
    );
}

#[test]
fn set_interpolation_rejects_unsupported_values() {
    let mut doc = Document::new(fixtures::single_quad());
    for (target, interpolation) in [
        (mesh("Quad"), KeyformInterpolation::Unknown),
        (KeyformTarget::Part("Root".into()), Smooth),
    ] {
        let error = doc
            .apply(set(target.clone(), None, interpolation))
            .unwrap_err();
        assert_eq!(
            error,
            EditError::UnsupportedInterpolation {
                target: target.to_string(),
                interpolation,
            }
        );
    }
    // パーツを線形にするのはかまわない (何もしない)。
    doc.apply(set(KeyformTarget::Part("Root".into()), None, Linear))
        .unwrap();
    let error = doc.apply(set(mesh("Missing"), None, Smooth)).unwrap_err();
    assert!(
        matches!(error, EditError::NotFound { kind: "mesh", .. }),
        "{error:?}"
    );
}

// 曲線補間の格子にキーを足すと、新しいセルの形は曲線の上に置く (足す前の見た目がキーの位置で変わらない)。
// 不透明度・描画順は線形の値。
#[test]
fn adding_a_key_to_a_smooth_grid_keeps_the_curve_value() {
    let model = fixtures::smooth_arc();
    let before = cheek(&model, 15.0);
    let mut doc = Document::new(model);
    doc.apply(Command::AddKeyformKey {
        target: mesh("Cheek"),
        parameter: "AngleX".into(),
        key: 15.0,
    })
    .unwrap();
    let grid = &doc.model().meshes[0].keyforms;
    assert_eq!(grid.bindings[0].keys, [-30.0, 0.0, 15.0, 30.0]);
    let added = &grid.keyforms[2];
    for (p, q) in added.positions.iter().zip(&before.0) {
        assert!((*p - *q).length() < 1e-3, "{p:?} != {q:?}");
    }
    assert_eq!((added.opacity, added.draw_order), (before.1, before.2));
    assert_eq!((before.1, before.2), (0.5, 550.0));
    let after = cheek(doc.model(), 15.0);
    assert_eq!(after, (added.positions.clone(), 0.5, 550.0));

    // 線形の格子では今までどおり、隣のキーの線形補間で埋める。
    let mut model = fixtures::smooth_arc();
    model.meshes[0].keyforms.interpolation = Linear;
    let linear = cheek(&model, 15.0);
    let mut doc = Document::new(model);
    doc.apply(Command::AddKeyformKey {
        target: mesh("Cheek"),
        parameter: "AngleX".into(),
        key: 15.0,
    })
    .unwrap();
    assert_eq!(
        doc.model().meshes[0].keyforms.keyforms[2].positions,
        linear.0
    );
}
