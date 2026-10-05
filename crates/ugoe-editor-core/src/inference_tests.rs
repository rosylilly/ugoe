//! 推定・ミラーのコマンドと、`derived` のセルの自動の作り直しのテスト
//! (`docs/spec/keyform-inference.md#推定の実行`)。

use ugoe_core::Vec2;
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::{KeyformGrid, KeyformInference, KeyformOrigin, Mesh, Model, fixtures};
use ugoe_rigging::{InferError, InferScope, MirrorOptions};

use crate::tests::{applied, binding, cell, rejected};
use crate::{
    CellError, Command, Document, EditError, InferMethod, KeyformTarget, MirrorReport,
    RederiveWarning,
};

use KeyformOrigin::{Additive, Authored, DepthRotation, Derived, Mirror};

mod infer;
mod mirror;
mod rederive;
mod reparent;

const EPS: f32 = 1e-5;
const KEYS: [f32; 3] = [-30.0, 0.0, 30.0];

fn mesh<'a>(model: &'a Model, id: &str) -> &'a Mesh {
    model.meshes.iter().find(|m| m.id == id).unwrap()
}

fn mesh_mut<'a>(model: &'a mut Model, id: &str) -> &'a mut Mesh {
    model.meshes.iter_mut().find(|m| m.id == id).unwrap()
}

fn target(id: &str) -> KeyformTarget {
    KeyformTarget::Mesh(id.into())
}

fn face() -> KeyformTarget {
    KeyformTarget::Deformer("Face".into())
}

fn shifted(kf: &MeshKeyform, dx: f32, dy: f32) -> MeshKeyform {
    let mut kf = kf.clone();
    for p in &mut kf.positions {
        *p += Vec2::new(dx, dy);
    }
    kf
}

#[track_caller]
fn assert_shift(actual: &MeshKeyform, base: &MeshKeyform, dx: f32, dy: f32) {
    for (a, b) in actual.positions.iter().zip(&base.positions) {
        let d = *a - *b;
        assert!(
            (d.x - dx).abs() < EPS && (d.y - dy).abs() < EPS,
            "shift {d:?}, expected ({dx}, {dy})"
        );
    }
}

/// 3×3 格子 (AngleX × AngleY、キー -30 0 30) のセル。
fn index(x: f32, y: f32) -> usize {
    let i = |v: f32| KEYS.iter().position(|&k| k == v).unwrap();
    i(x) + 3 * i(y)
}

fn xy(x: f32, y: f32) -> crate::KeyformCell {
    cell(&[("AngleX", x), ("AngleY", y)])
}

/// `keyform_inference` の `EyeL` を AngleX × AngleY の 3×3 格子にしたモデル (加算合成)。
///
/// 基準と 1 軸のセルが `authored` (AngleX = -30 で x −0.05、30 で x +0.02、AngleY = -30 で
/// y −0.01、30 で y +0.03)。角のセルは `derived` で、値は基準のまま (まだ推定していない)。
fn stale_model() -> Model {
    let mut model = fixtures::keyform_inference();
    let eye = mesh_mut(&mut model, "EyeL");
    let base = eye.keyforms.keyforms[0].clone();
    let mut keyforms = vec![base.clone(); 9];
    keyforms[index(-30.0, 0.0)] = shifted(&base, -0.05, 0.0);
    keyforms[index(30.0, 0.0)] = shifted(&base, 0.02, 0.0);
    keyforms[index(0.0, -30.0)] = shifted(&base, 0.0, -0.01);
    keyforms[index(0.0, 30.0)] = shifted(&base, 0.0, 0.03);
    let mut grid = KeyformGrid::new(
        vec![binding("AngleX", &KEYS), binding("AngleY", &KEYS)],
        keyforms,
    );
    grid.origins = (0..9)
        .map(|i| {
            if i % 2 == 1 || i == 4 {
                Authored
            } else {
                Derived
            }
        })
        .collect();
    eye.keyforms = grid;
    model
}

/// [`stale_model`] の角のセルを推定済みにしたモデル。
fn model() -> Model {
    let mut model = stale_model();
    ugoe_rigging::infer_additive_target(
        &mut model,
        &(&target("EyeL")).into(),
        InferScope::AllDerived,
    )
    .unwrap();
    model
}

fn base(model: &Model) -> MeshKeyform {
    mesh(model, "EyeL").keyforms.keyforms[index(0.0, 0.0)].clone()
}

fn eye_cell(model: &Model, x: f32, y: f32) -> &MeshKeyform {
    &mesh(model, "EyeL").keyforms.keyforms[index(x, y)]
}

fn eye_origin(model: &Model, x: f32, y: f32) -> KeyformOrigin {
    mesh(model, "EyeL").keyforms.origin(index(x, y))
}

fn inference_error(target: &KeyformTarget, error: InferError) -> EditError {
    EditError::Inference {
        target: target.to_string(),
        error,
    }
}

/// [`model`] の `EyeR` を同じ軸の 3×3 格子 (基準だけ `authored`) にしたモデル。
fn mirror_model() -> Model {
    let mut model = model();
    let eye = mesh_mut(&mut model, "EyeR");
    let base = eye.keyforms.keyforms[0].clone();
    eye.keyforms = KeyformGrid::new(
        vec![binding("AngleX", &KEYS), binding("AngleY", &KEYS)],
        vec![base; 9],
    );
    eye.keyforms.origins = vec![Derived; 9];
    eye.keyforms.origins[4] = Authored;
    model
}

/// [`mirror_model`] の `EyeR` を `EyeL` からミラーし、角 (AngleX = 30, AngleY = 30) を加算合成の
/// セルにしたモデル。`EyeR` は基準が `authored`、角が `additive`、それ以外が `mirror`。
fn mirrored_model() -> Model {
    let mut doc = Document::new(mirror_model());
    doc.apply(Command::MirrorKeyforms {
        source: target("EyeL"),
        target: target("EyeR"),
        axis: None,
        tolerance: None,
        overwrite_authored: false,
    })
    .unwrap();
    doc.apply(Command::MarkKeyformDerived {
        target: target("EyeR"),
        cells: vec![xy(30.0, 30.0)],
    })
    .unwrap();
    doc.model().clone()
}

fn right_cell(model: &Model, x: f32, y: f32) -> &MeshKeyform {
    &mesh(model, "EyeR").keyforms.keyforms[index(x, y)]
}

fn right_origin(model: &Model, x: f32, y: f32) -> KeyformOrigin {
    mesh(model, "EyeR").keyforms.origin(index(x, y))
}

/// `doc` に `command` を適用し、警告を返す (コマンドは拒否されない)。
fn warnings_of(model: &Model, command: Command) -> (Model, Vec<RederiveWarning>) {
    let mut doc = Document::new(model.clone());
    let report = doc.apply_with_report(command).unwrap();
    (doc.model().clone(), report.warnings)
}

fn face_grid(model: &Model) -> &KeyformGrid<ugoe_model::deformer::WarpKeyform> {
    match &model.deformers[0].kind {
        ugoe_model::DeformerKind::Warp(warp) => &warp.keyforms,
        _ => panic!("Face is a warp deformer"),
    }
}
