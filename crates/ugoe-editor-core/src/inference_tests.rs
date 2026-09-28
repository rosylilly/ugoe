//! 推定・ミラーのコマンドと、`derived` のセルの自動の作り直しのテスト
//! (`docs/spec/keyform-inference.md#推定の実行`)。

use ugoe_core::Vec2;
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::{KeyformGrid, KeyformInference, KeyformOrigin, Mesh, Model, fixtures};
use ugoe_rigging::{InferError, MirrorOptions};

use crate::tests::{applied, binding, cell, rejected};
use crate::{CellError, Command, Document, EditError, InferMethod, KeyformTarget};

use KeyformOrigin::{Authored, Derived};

const EPS: f32 = 1e-5;
const KEYS: [f32; 3] = [-30.0, 0.0, 30.0];

fn mesh<'a>(model: &'a Model, id: &str) -> &'a Mesh {
    model.meshes.iter().find(|m| m.id == id.into()).unwrap()
}

fn mesh_mut<'a>(model: &'a mut Model, id: &str) -> &'a mut Mesh {
    model.meshes.iter_mut().find(|m| m.id == id.into()).unwrap()
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
    ugoe_rigging::infer_additive_target(&mut model, &(&target("EyeL")).into()).unwrap();
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

// ---- InferKeyforms ----

#[test]
fn infer_keyforms_additive() {
    let model = stale_model();
    let base = base(&model);
    for method in [InferMethod::FromGrid, InferMethod::Additive] {
        let after = applied(
            &model,
            Command::InferKeyforms {
                target: target("EyeL"),
                method,
            },
        );
        assert_shift(eye_cell(&after, 30.0, 30.0), &base, 0.02, 0.03);
        assert_shift(eye_cell(&after, -30.0, -30.0), &base, -0.05, -0.01);
        assert_eq!(eye_origin(&after, 30.0, 30.0), Derived);
    }
}

#[test]
fn infer_keyforms_depth_rotation_on_fixture() {
    let model = fixtures::keyform_inference();
    let mut expected = model.clone();
    ugoe_rigging::infer_depth_rotation(&mut expected, &(&face()).into()).unwrap();
    assert_ne!(expected, model, "the fixture's derived cells must change");
    for method in [InferMethod::FromGrid, InferMethod::DepthRotation] {
        let after = applied(
            &model,
            Command::InferKeyforms {
                target: face(),
                method,
            },
        );
        assert_eq!(after, expected);
    }
}

#[test]
fn infer_keyforms_rejects_invalid_targets() {
    let model = stale_model();
    assert_eq!(
        rejected(
            &model,
            Command::InferKeyforms {
                target: target("Missing"),
                method: InferMethod::FromGrid,
            },
        ),
        EditError::NotFound {
            kind: "mesh",
            id: "Missing".into(),
        }
    );
    // 推定の設定が加算合成の格子は深度から回せない。
    let mut additive_face = model.clone();
    if let ugoe_model::DeformerKind::Warp(warp) = &mut additive_face.deformers[0].kind {
        warp.keyforms.inference = None;
    }
    assert_eq!(
        rejected(
            &additive_face,
            Command::InferKeyforms {
                target: face(),
                method: InferMethod::DepthRotation,
            },
        ),
        inference_error(&face(), InferError::NotDepthRotation((&face()).into()))
    );
    // 親がワープ (UV 空間) の目は深度から回せない。
    assert_eq!(
        rejected(
            &model,
            Command::InferKeyforms {
                target: target("EyeL"),
                method: InferMethod::DepthRotation,
            },
        ),
        inference_error(
            &target("EyeL"),
            InferError::NonAffineParent((&target("EyeL")).into())
        )
    );

    let mut base_derived = model.clone();
    mesh_mut(&mut base_derived, "EyeL").keyforms.origins[4] = Derived;
    assert_eq!(
        rejected(
            &base_derived,
            Command::InferKeyforms {
                target: target("EyeL"),
                method: InferMethod::Additive,
            },
        ),
        inference_error(&target("EyeL"), InferError::BaseNotAuthored { cell: 4 })
    );

    let mut unknown = model.clone();
    mesh_mut(&mut unknown, "EyeL").keyforms.inference = Some(KeyformInference::Unknown);
    assert_eq!(
        rejected(
            &unknown,
            Command::InferKeyforms {
                target: target("EyeL"),
                method: InferMethod::FromGrid,
            },
        ),
        EditError::UnsupportedInference {
            target: "mesh `EyeL`".into(),
        }
    );
}

// ---- MirrorKeyforms ----

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

#[test]
fn mirror_keyforms_through_command() {
    let model = mirror_model();
    let mut expected = model.clone();
    ugoe_rigging::mirror_keyforms(
        &mut expected,
        &(&target("EyeL")).into(),
        &(&target("EyeR")).into(),
        &MirrorOptions::default(),
    )
    .unwrap();
    let after = applied(
        &model,
        Command::MirrorKeyforms {
            source: target("EyeL"),
            target: target("EyeR"),
            axis: None,
            tolerance: None,
            overwrite_authored: false,
        },
    );
    assert_eq!(after, expected);
    let right = &mesh(&after, "EyeR").keyforms;
    assert_eq!(right.origin(4), Authored);
    assert_eq!(right.origin(index(-30.0, 0.0)), Derived);
    // AngleX = -30 の左目 (x −0.05) を反転すると、右目は x +0.05。
    assert_shift(
        &right.keyforms[index(-30.0, 0.0)],
        &right.keyforms[4],
        0.05,
        0.0,
    );
}

#[test]
fn mirror_keyforms_passes_options() {
    let model = mirror_model();
    let mut expected = model.clone();
    let options = MirrorOptions {
        axis: Some(0.5),
        tolerance: Some(0.01),
        overwrite_authored: true,
    };
    ugoe_rigging::mirror_keyforms(
        &mut expected,
        &(&target("EyeL")).into(),
        &(&target("EyeR")).into(),
        &options,
    )
    .unwrap();
    let after = applied(
        &model,
        Command::MirrorKeyforms {
            source: target("EyeL"),
            target: target("EyeR"),
            axis: options.axis,
            tolerance: options.tolerance,
            overwrite_authored: true,
        },
    );
    assert_eq!(after, expected);
    assert_eq!(mesh(&after, "EyeR").keyforms.origins, vec![Derived; 9]);
}

#[test]
fn mirror_keyforms_rejects_invalid_pairs() {
    let model = mirror_model();
    let mirror = |source: &str, to: &str| Command::MirrorKeyforms {
        source: target(source),
        target: target(to),
        axis: None,
        tolerance: None,
        overwrite_authored: false,
    };
    assert_eq!(
        rejected(&model, mirror("EyeL", "Missing")),
        EditError::NotFound {
            kind: "mesh",
            id: "Missing".into(),
        }
    );
    assert_eq!(
        rejected(&model, mirror("EyeL", "Nose")),
        inference_error(
            &target("Nose"),
            InferError::NotMirrorPair {
                from: (&target("EyeL")).into(),
                to: (&target("Nose")).into(),
            }
        )
    );
}

// ---- MarkKeyformAuthored / MarkKeyformDerived ----

#[test]
fn mark_keyform_derived_reinfers_the_cells() {
    let model = model();
    let base = base(&model);
    let after = applied(
        &model,
        Command::MarkKeyformDerived {
            target: target("EyeL"),
            cells: vec![xy(30.0, 0.0)],
        },
    );
    assert_eq!(eye_origin(&after, 30.0, 0.0), Derived);
    // AngleX の正の側に `authored` のセルがないので、基準と -30 (x −0.05) から外挿する。
    assert_shift(eye_cell(&after, 30.0, 0.0), &base, 0.05, 0.0);
    // 角のセルも作り直した 1 軸のセルを使う。
    assert_shift(eye_cell(&after, 30.0, 30.0), &base, 0.05, 0.03);
}

#[test]
fn mark_keyform_derived_on_a_grid_without_origins() {
    let mut model = model();
    let eye = mesh_mut(&mut model, "EyeL");
    eye.keyforms.origins.clear();
    let after = applied(
        &model,
        Command::MarkKeyformDerived {
            target: target("EyeL"),
            cells: vec![xy(30.0, 30.0)],
        },
    );
    let mut origins = vec![Authored; 9];
    origins[index(30.0, 30.0)] = Derived;
    assert_eq!(mesh(&after, "EyeL").keyforms.origins, origins);
}

#[test]
fn mark_keyform_derived_rejects_the_base_cell() {
    assert_eq!(
        rejected(
            &model(),
            Command::MarkKeyformDerived {
                target: target("EyeL"),
                cells: vec![xy(0.0, 0.0)],
            },
        ),
        inference_error(&target("EyeL"), InferError::BaseNotAuthored { cell: 4 })
    );
}

#[test]
fn mark_keyform_authored_keeps_values() {
    let model = model();
    let after = applied(
        &model,
        Command::MarkKeyformAuthored {
            target: target("EyeL"),
            cells: vec![xy(30.0, 30.0), xy(-30.0, -30.0)],
        },
    );
    assert_eq!(eye_origin(&after, 30.0, 30.0), Authored);
    assert_eq!(eye_origin(&after, -30.0, -30.0), Authored);
    assert_eq!(eye_origin(&after, 30.0, -30.0), Derived);
    let eye = &mesh(&after, "EyeL").keyforms;
    assert_eq!(eye.keyforms, mesh(&model, "EyeL").keyforms.keyforms);

    // 以後、`authored` にした角のセルは作り直さない。
    let mut doc = Document::new(after);
    let keyform = shifted(&base(&model), 0.1, 0.0);
    doc.apply(Command::SetMeshKeyform {
        id: "EyeL".into(),
        cell: xy(30.0, 0.0),
        keyform,
    })
    .unwrap();
    assert_eq!(
        eye_cell(doc.model(), 30.0, 30.0),
        eye_cell(&model, 30.0, 30.0)
    );
    assert_shift(
        eye_cell(doc.model(), 30.0, -30.0),
        &base(&model),
        0.1,
        -0.01,
    );
}

#[test]
fn mark_keyform_rejects_invalid_cells_and_targets() {
    let model = model();
    let err = rejected(
        &model,
        Command::MarkKeyformAuthored {
            target: target("EyeL"),
            cells: vec![cell(&[("AngleX", 30.0)])],
        },
    );
    assert_eq!(
        err,
        EditError::InvalidCell {
            kind: "keyform grid",
            id: "mesh `EyeL`".into(),
            error: CellError::MissingAxis("AngleY".into()),
        }
    );
    assert_eq!(
        rejected(
            &model,
            Command::MarkKeyformDerived {
                target: KeyformTarget::Glue("Missing".into()),
                cells: vec![],
            },
        ),
        EditError::NotFound {
            kind: "glue",
            id: "Missing".into(),
        }
    );
}

// ---- 自動の作り直し ----

#[test]
fn editing_an_authored_cell_rederives_in_the_same_undo_step() {
    let model = model();
    let base = base(&model);
    // `applied` は 1 回の Undo で編集前のモデル (角のセルを含む) に戻ることも確かめる。
    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "EyeL".into(),
            cell: xy(30.0, 0.0),
            keyform: shifted(&base, 0.1, 0.0),
        },
    );
    assert_eq!(eye_origin(&after, 30.0, 0.0), Authored);
    assert_shift(eye_cell(&after, 30.0, 30.0), &base, 0.1, 0.03);
    assert_shift(eye_cell(&after, 30.0, -30.0), &base, 0.1, -0.01);
    // AngleX = -30 側の角は変わらない。
    assert_eq!(eye_cell(&after, -30.0, 30.0), eye_cell(&model, -30.0, 30.0));
}

#[test]
fn editing_a_derived_cell_makes_it_authored() {
    let model = model();
    let base = base(&model);
    let mut doc = Document::new(model.clone());
    doc.apply(Command::SetMeshKeyform {
        id: "EyeL".into(),
        cell: xy(30.0, 30.0),
        keyform: shifted(&base, 0.2, 0.2),
    })
    .unwrap();
    assert_eq!(eye_origin(doc.model(), 30.0, 30.0), Authored);
    assert_shift(eye_cell(doc.model(), 30.0, 30.0), &base, 0.2, 0.2);

    // 元のセルを編集しても、手で直した角は上書きされない。
    doc.apply(Command::SetMeshKeyform {
        id: "EyeL".into(),
        cell: xy(0.0, 30.0),
        keyform: shifted(&base, 0.0, 0.1),
    })
    .unwrap();
    assert_shift(eye_cell(doc.model(), 30.0, 30.0), &base, 0.2, 0.2);
    assert_shift(eye_cell(doc.model(), -30.0, 30.0), &base, -0.05, 0.1);

    assert!(doc.undo());
    assert!(doc.undo());
    assert_eq!(doc.model(), &model);
}

#[test]
fn replacing_a_grid_makes_edited_derived_cells_authored() {
    let model = model();
    let base = base(&model);
    let mut keyforms = mesh(&model, "EyeL").keyforms.clone();
    keyforms.keyforms[index(30.0, 30.0)] = shifted(&base, 0.2, 0.2);
    keyforms.keyforms[index(0.0, 30.0)] = shifted(&base, 0.0, 0.1);
    let after = applied(
        &model,
        Command::SetMeshKeyforms {
            id: "EyeL".into(),
            keyforms,
        },
    );
    assert_eq!(eye_origin(&after, 30.0, 30.0), Authored);
    assert_shift(eye_cell(&after, 30.0, 30.0), &base, 0.2, 0.2);
    // 変えていない `derived` のセルは `derived` のまま作り直される。
    assert_eq!(eye_origin(&after, -30.0, 30.0), Derived);
    assert_shift(eye_cell(&after, -30.0, 30.0), &base, -0.05, 0.1);
}

#[test]
fn rederivation_failure_rejects_the_edit() {
    let model = model();
    let mut keyforms = mesh(&model, "EyeL").keyforms.clone();
    keyforms.origins[4] = Derived;
    assert_eq!(
        rejected(
            &model,
            Command::SetMeshKeyforms {
                id: "EyeL".into(),
                keyforms,
            },
        ),
        inference_error(&target("EyeL"), InferError::BaseNotAuthored { cell: 4 })
    );
}

#[test]
fn depth_rotation_grids_rederive_with_their_own_method() {
    let mut model = fixtures::keyform_inference();
    ugoe_rigging::infer_depth_rotation(&mut model, &(&face()).into()).unwrap();
    let mut keyforms = match &model.deformers[0].kind {
        ugoe_model::DeformerKind::Warp(warp) => warp.keyforms.clone(),
        _ => panic!("Face is a warp deformer"),
    };
    // AngleX = 30 (セル 5) を手で直すと、斜めの AngleX = 30, AngleY = 30 (セル 8) にも同じ修正が入る。
    let before_diagonal = keyforms.keyforms[8].clone();
    for p in &mut keyforms.keyforms[5].points {
        p.x += 4.0;
    }
    let after = applied(
        &model,
        Command::SetWarpKeyforms {
            id: "Face".into(),
            keyforms,
        },
    );
    let ugoe_model::DeformerKind::Warp(warp) = &after.deformers[0].kind else {
        panic!("Face is a warp deformer");
    };
    assert_eq!(warp.keyforms.origin(5), Authored);
    assert_eq!(warp.keyforms.origin(8), Derived);
    for (a, b) in warp.keyforms.keyforms[8]
        .points
        .iter()
        .zip(&before_diagonal.points)
    {
        assert!((a.x - b.x - 4.0).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3);
    }
}

#[test]
fn batch_rederives_once_after_all_commands() {
    let model = model();
    let base = base(&model);
    let after = applied(
        &model,
        Command::Batch {
            label: "edit".into(),
            commands: vec![
                Command::SetMeshKeyform {
                    id: "EyeL".into(),
                    cell: xy(30.0, 0.0),
                    keyform: shifted(&base, 0.1, 0.0),
                },
                Command::SetMeshKeyform {
                    id: "EyeL".into(),
                    cell: xy(0.0, 30.0),
                    keyform: shifted(&base, 0.0, 0.1),
                },
            ],
        },
    );
    assert_shift(eye_cell(&after, 30.0, 30.0), &base, 0.1, 0.1);
}

#[test]
fn inference_commands_json() {
    let cmd: Command =
        serde_json::from_str(r#"{"op":"infer_keyforms","target":{"kind":"deformer","id":"Face"}}"#)
            .unwrap();
    assert_eq!(
        cmd,
        Command::InferKeyforms {
            target: face(),
            method: InferMethod::FromGrid,
        }
    );
    let cmd: Command = serde_json::from_str(
        r#"{"op":"mirror_keyforms","source":{"kind":"mesh","id":"EyeL"},
            "target":{"kind":"mesh","id":"EyeR"}}"#,
    )
    .unwrap();
    assert_eq!(
        cmd,
        Command::MirrorKeyforms {
            source: target("EyeL"),
            target: target("EyeR"),
            axis: None,
            tolerance: None,
            overwrite_authored: false,
        }
    );
    let cmd: Command = serde_json::from_str(
        r#"{"op":"mark_keyform_derived","target":{"kind":"glue","id":"G"},
            "cells":[{"AngleX":30}]}"#,
    )
    .unwrap();
    assert_eq!(
        cmd,
        Command::MarkKeyformDerived {
            target: KeyformTarget::Glue("G".into()),
            cells: vec![cell(&[("AngleX", 30.0)])],
        }
    );
}
