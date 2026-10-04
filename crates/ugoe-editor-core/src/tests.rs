//! `Document::apply` と各コマンドのテスト。
//!
//! 入力は `ugoe_model::fixtures` のモデルを使う。新しい要素の値も、構造体リテラルではなく
//! フィクスチャの要素を複製して作る (モデルの型にフィールドが増えても直す箇所を減らすため)。

use ugoe_model::deformer::DeformerKind;
use ugoe_model::validate::{ValidationError, validate};
use ugoe_model::{
    BlendMode, Deformer, HitArea, KeyformBinding, KeyformGrid, MaskRef, Mesh, Model, Parameter,
    ParameterGroup, Part, fixtures,
};

use crate::{CellError, ChildRef, Command, Document, EditError, KeyformCell};

mod batch;
mod deformer;
mod json;
mod mesh;
mod parameter;
mod part;

// ---- 要素を作る補助 ----

fn new_parameter(id: &str, min: f32, default: f32, max: f32) -> Parameter {
    let mut p = fixtures::single_quad().parameters.remove(0);
    p.id = id.into();
    p.name = id.into();
    (p.min, p.default, p.max) = (min, default, max);
    p
}

fn new_part(id: &str, parent: Option<&str>) -> Part {
    let mut p = fixtures::single_quad().parts.remove(0);
    p.id = id.into();
    p.name = id.into();
    p.parent = parent.map(Into::into);
    p
}

/// パラメータ `X` で左右に動く四角形 (`single_quad` の `Quad`)。パーツ `Root`、親なし。
fn new_mesh(id: &str) -> Mesh {
    let mut m = fixtures::single_quad().meshes.remove(0);
    m.id = id.into();
    m.name = id.into();
    m
}

/// 1×1 セルの定数ワープ (`rotation_chain` の `Cloth`)。パーツ `Root`。
fn new_warp(id: &str, parent: Option<&str>) -> Deformer {
    let mut d = rotation_chain_deformer("Cloth");
    d.id = id.into();
    d.name = id.into();
    d.parent = parent.map(Into::into);
    d
}

fn rotation_chain_deformer(id: &str) -> Deformer {
    fixtures::rotation_chain()
        .deformers
        .into_iter()
        .find(|d| d.id.as_str() == id)
        .unwrap()
}

pub(crate) fn binding(parameter: &str, keys: &[f32]) -> KeyformBinding {
    KeyformBinding {
        parameter: parameter.into(),
        keys: keys.to_vec(),
    }
}

/// 1 軸の格子。
fn grid1<T: Clone>(parameter: &str, keys: &[f32], keyforms: Vec<T>) -> KeyformGrid<T> {
    let mut grid = KeyformGrid::constant(keyforms[0].clone());
    grid.bindings = vec![binding(parameter, keys)];
    grid.keyforms = keyforms;
    grid
}

pub(crate) fn cell(entries: &[(&str, f32)]) -> KeyformCell {
    entries.iter().map(|&(p, v)| (p.into(), v)).collect()
}

fn mesh_ids(model: &Model) -> Vec<&str> {
    model.meshes.iter().map(|m| m.id.as_str()).collect()
}

// ---- 検査の補助 ----

/// 適用に成功し、結果が検証を通り、Undo / Redo で前後を行き来できることを確かめて、適用後のモデルを返す。
#[track_caller]
pub(crate) fn applied(model: &Model, command: Command) -> Model {
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
pub(crate) fn rejected(model: &Model, command: Command) -> EditError {
    let mut doc = Document::new(model.clone());
    let err = doc.apply(command).unwrap_err();
    assert_eq!(doc.model(), model);
    assert!(!doc.undo());
    err
}

/// 検証エラーで拒否され、その中に `pred` を満たすものがあること。
#[track_caller]
fn rejected_invalid(model: &Model, command: Command, pred: impl Fn(&ValidationError) -> bool) {
    match rejected(model, command) {
        EditError::Invalid(errors) => assert!(errors.iter().any(pred), "{errors:?}"),
        other => panic!("expected validation error, got {other:?}"),
    }
}

fn not_found(kind: &'static str, id: &str) -> EditError {
    EditError::NotFound {
        kind,
        id: id.into(),
    }
}
