//! デフォーマの作成・見た目を保つ付け替え・セルの編集のテスト。式は `docs/editor.md#デフォーマの編集`。
//!
//! 見た目の比較は、ランタイムで評価したメッシュのキャンバス座標を、適用の前後でいくつかの
//! パラメータ値について比べる。

use ugoe_core::Vec2;
use ugoe_model::deformer::{DeformerKind, RotationKeyform};
use ugoe_model::validate::validate;
use ugoe_model::{Deformer, KeyformOrigin, MirrorSource, Model, fixtures};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::{ChildRef, Command, Document, EditError};

mod blend_shape;
mod create;
mod rejection;
mod reparent;
mod rotation_origin;
mod unwrapping;

const TOL: f32 = 2e-3;

// ---- 補助 ----

/// 適用に成功し、結果が検証を通り、Undo / Redo で前後を行き来できることを確かめて、適用後のモデルを返す。
#[track_caller]
fn applied(model: &Model, command: Command) -> Model {
    assert!(validate(model).is_empty(), "fixture must be valid");
    let mut doc = Document::new(model.clone());
    doc.apply(command).unwrap();
    let after = doc.model().clone();
    assert_eq!(validate(&after), vec![]);
    assert!(doc.undo());
    assert_eq!(doc.model(), model);
    assert!(doc.redo());
    assert_eq!(doc.model(), &after);
    after
}

/// 適用に失敗し、モデルと履歴が変わらないことを確かめて、エラーを返す。
#[track_caller]
fn rejected(model: &Model, command: Command) -> EditError {
    let mut doc = Document::new(model.clone());
    let err = doc.apply(command).unwrap_err();
    assert_eq!(doc.model(), model);
    assert!(!doc.undo());
    err
}

/// パラメータ値 (`(ID, 値)`、ほかは既定値) で評価した全メッシュのキャンバス座標。
fn evaluate(model: &Model, values: &[(&str, f32)]) -> Vec<Vec<Vec2>> {
    let mut instance = ModelInstance::new(CompiledModel::compile(model.clone()).unwrap());
    for &(id, value) in values {
        let index = instance.parameters().index_of(id).unwrap();
        instance.parameters_mut().set(index, value);
    }
    instance.update();
    instance
        .drawables()
        .iter()
        .map(|d| d.positions.clone())
        .collect()
}

/// 各パラメータ値で、前後の見た目 (全メッシュの頂点) が許容誤差内で一致することを確かめる。
#[track_caller]
fn assert_same_look(before: &Model, after: &Model, poses: &[&[(&str, f32)]]) {
    for pose in poses {
        let (a, b) = (evaluate(before, pose), evaluate(after, pose));
        assert_eq!(a.len(), b.len());
        for (i, (ma, mb)) in a.iter().zip(&b).enumerate() {
            for (p, q) in ma.iter().zip(mb) {
                assert!(
                    p.abs_diff_eq(*q, TOL),
                    "pose {pose:?}, mesh {}: {p} != {q}",
                    before.meshes[i].id
                );
            }
        }
    }
}

const REST: &[(&str, f32)] = &[];

fn deformer<'a>(model: &'a Model, id: &str) -> &'a Deformer {
    model.deformers.iter().find(|d| d.id == id).unwrap()
}

fn mesh_parent(model: &Model, id: &str) -> Option<String> {
    model
        .meshes
        .iter()
        .find(|m| m.id == id)
        .unwrap()
        .parent
        .as_ref()
        .map(ToString::to_string)
}

fn mesh(id: &str) -> ChildRef {
    ChildRef::Mesh(id.into())
}

fn def(id: &str) -> ChildRef {
    ChildRef::Deformer(id.into())
}

fn warp(children: Vec<ChildRef>, rows: u16, cols: u16) -> Command {
    Command::CreateWarpDeformer {
        id: Some("New".into()),
        name: "New".into(),
        children,
        parts: Vec::new(),
        rows,
        cols,
        parent: None,
    }
}

fn rotation(children: Vec<ChildRef>, origin: Option<Vec2>) -> Command {
    Command::CreateRotationDeformer {
        id: Some("New".into()),
        name: "New".into(),
        children,
        parts: Vec::new(),
        origin,
        parent: None,
    }
}

fn reparent(element: ChildRef, new_parent: Option<&str>) -> Command {
    Command::ReparentPreservingLook {
        element,
        new_parent: new_parent.map(Into::into),
    }
}

fn set_default(model: &mut Model, id: &str, value: f32) {
    let p = model.parameters.iter_mut().find(|p| p.id == id).unwrap();
    p.default = value;
}

fn warp_points(model: &Model, id: &str) -> Vec<Vec2> {
    match &deformer(model, id).kind {
        DeformerKind::Warp(w) => w.keyforms.keyforms[0].points.clone(),
        _ => panic!("not a warp"),
    }
}

fn rotation_keyforms(model: &Model, id: &str) -> Vec<RotationKeyform> {
    match &deformer(model, id).kind {
        DeformerKind::Rotation(r) => r.keyforms.keyforms.clone(),
        _ => panic!("not a rotation deformer"),
    }
}

fn unwrap(id: &str) -> Command {
    Command::UnwrapDeformer { id: id.into() }
}
