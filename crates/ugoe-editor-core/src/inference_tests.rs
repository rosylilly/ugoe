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
                overwrite_derived: false,
            },
        );
        assert_shift(eye_cell(&after, 30.0, 30.0), &base, 0.02, 0.03);
        assert_shift(eye_cell(&after, -30.0, -30.0), &base, -0.05, -0.01);
        assert_eq!(eye_origin(&after, 30.0, 30.0), Additive);
    }
}

#[test]
fn infer_keyforms_depth_rotation_on_fixture() {
    let model = fixtures::keyform_inference();
    let mut expected = model.clone();
    ugoe_rigging::infer_depth_rotation(&mut expected, &(&face()).into(), InferScope::AllDerived)
        .unwrap();
    assert_ne!(expected, model, "the fixture's derived cells must change");
    for method in [InferMethod::FromGrid, InferMethod::DepthRotation] {
        let after = applied(
            &model,
            Command::InferKeyforms {
                target: face(),
                method,
                overwrite_derived: false,
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
                overwrite_derived: false,
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
                overwrite_derived: false,
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
                overwrite_derived: false,
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
                overwrite_derived: false,
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
                overwrite_derived: false,
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
    assert_eq!(right.origin(index(-30.0, 0.0)), Mirror);
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
        scope: InferScope::AllDerived,
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
    assert_eq!(mesh(&after, "EyeR").keyforms.origins, vec![Mirror; 9]);
}

/// [`mirror_model`] の `EyeR` のセルをすべて `authored` にしたモデル (キーを追加しただけの格子)。
/// `edited` のセルだけは基準から動かしてある。
fn authored_target_model(edited: &[(f32, f32)]) -> Model {
    let mut model = mirror_model();
    let eye = mesh_mut(&mut model, "EyeR");
    eye.keyforms.origins = Vec::new();
    let base = eye.keyforms.keyforms[4].clone();
    for &(x, y) in edited {
        eye.keyforms.keyforms[index(x, y)] = shifted(&base, 0.1, 0.0);
    }
    model
}

fn mirror_eyes(overwrite_authored: bool) -> Command {
    Command::MirrorKeyforms {
        source: target("EyeL"),
        target: target("EyeR"),
        axis: None,
        tolerance: None,
        overwrite_authored,
    }
}

/// `command` を適用し、ミラーの結果を返す。
fn mirror_report(model: &Model, command: Command) -> (Model, Vec<MirrorReport>) {
    let mut doc = Document::new(model.clone());
    let report = doc.apply_with_report(command).unwrap();
    (doc.model().clone(), report.mirrored)
}

#[test]
fn mirror_overwrites_unedited_authored_cells() {
    let model = authored_target_model(&[]);
    let (after, reports) = mirror_report(&model, mirror_eyes(false));
    let right = &mesh(&after, "EyeR").keyforms;
    // 基準のセルは `authored` のまま、キーの追加で埋めただけのセルはミラーする。
    assert_eq!(right.origin(4), Authored);
    for i in (0..9).filter(|&i| i != 4) {
        assert_eq!(right.origin(i), Mirror, "cell {i}");
    }
    assert_shift(
        &right.keyforms[index(-30.0, 0.0)],
        &right.keyforms[4],
        0.05,
        0.0,
    );
    assert_eq!(
        reports,
        vec![MirrorReport {
            source: target("EyeL"),
            target: target("EyeR"),
            written: 8,
            unedited: 8,
            kept_authored: 1,
        }]
    );
}

#[test]
fn mirror_keeps_edited_authored_cells_and_reports_them() {
    let model = authored_target_model(&[(-30.0, 0.0)]);
    let (after, reports) = mirror_report(&model, mirror_eyes(false));
    let right = &mesh(&after, "EyeR").keyforms;
    assert_eq!(right.origin(index(-30.0, 0.0)), Authored);
    assert_eq!(
        right.keyforms[index(-30.0, 0.0)],
        mesh(&model, "EyeR").keyforms.keyforms[index(-30.0, 0.0)]
    );
    assert_eq!(right.origin(index(30.0, 0.0)), Mirror);
    assert_eq!(
        (
            reports[0].written,
            reports[0].unedited,
            reports[0].kept_authored
        ),
        (7, 7, 2)
    );
    // `overwrite_authored` なら、編集したセルも基準のセルも上書きする。
    let (after, reports) = mirror_report(&model, mirror_eyes(true));
    assert_eq!(mesh(&after, "EyeR").keyforms.origins, vec![Mirror; 9]);
    assert_eq!(
        (
            reports[0].written,
            reports[0].unedited,
            reports[0].kept_authored
        ),
        (9, 0, 0)
    );
}

#[test]
fn mirror_that_writes_nothing_reports_the_kept_cells() {
    let edited: Vec<(f32, f32)> = KEYS
        .iter()
        .flat_map(|&x| KEYS.iter().map(move |&y| (x, y)))
        .filter(|&(x, y)| (x, y) != (0.0, 0.0))
        .collect();
    let model = authored_target_model(&edited);
    let (after, reports) = mirror_report(&model, mirror_eyes(false));
    // 何も書かなければ、由来もミラー元の記録も変えない。
    assert_eq!(after, model);
    assert_eq!(
        (
            reports[0].written,
            reports[0].unedited,
            reports[0].kept_authored
        ),
        (0, 0, 9)
    );
}

#[test]
fn self_mirror_fills_unedited_cells_from_edited_ones() {
    // ワープ `Face` を AngleX だけの格子 (キー -30 0 30、すべて `authored`) にし、30 のセルだけを
    // 編集する。ワープの制御点は並びで対応するので、左右対称な 1 つの要素としてミラーできる。
    let mut model = fixtures::keyform_inference();
    model
        .parameters
        .iter_mut()
        .find(|p| p.id.as_str() == "AngleX")
        .unwrap()
        .mirror = Some("AngleX".into());
    let ugoe_model::DeformerKind::Warp(warp) = &mut model.deformers[0].kind else {
        panic!("Face is a warp deformer");
    };
    let base = warp.keyforms.keyforms[0].clone();
    let mut turned = base.clone();
    for p in &mut turned.points {
        p.x += 0.02;
    }
    warp.keyforms = KeyformGrid::new(
        vec![binding("AngleX", &KEYS)],
        vec![base.clone(), base, turned],
    );
    let (after, reports) = mirror_report(
        &model,
        Command::MirrorKeyforms {
            source: face(),
            target: face(),
            axis: None,
            tolerance: None,
            overwrite_authored: false,
        },
    );
    assert_eq!(face_grid(&after).origins, vec![Mirror, Authored, Authored]);
    // 編集した 30 のセルは、`overwrite_authored` なら -30 の反転で上書きされる (残したセル)。
    assert_eq!(
        (
            reports[0].written,
            reports[0].unedited,
            reports[0].kept_authored
        ),
        (1, 1, 1)
    );
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
    assert_eq!(eye_origin(&after, 30.0, 0.0), Additive);
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
    // 格子の方法 (加算合成) で作り直し、方法を記録する。
    let mut origins = vec![Authored; 9];
    origins[index(30.0, 30.0)] = Additive;
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
    assert_eq!(eye_origin(&after, 30.0, -30.0), Additive);
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
    assert_eq!(eye_origin(&after, -30.0, 30.0), Additive);
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
    ugoe_rigging::infer_depth_rotation(&mut model, &(&face()).into(), InferScope::AllDerived)
        .unwrap();
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
    assert_eq!(warp.keyforms.origin(8), DepthRotation);
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
            overwrite_derived: false,
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

// ---- セルの由来 (推定の方法) ごとの作り直し ----

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

#[test]
fn mirror_records_the_method_and_the_source() {
    let model = mirrored_model();
    let right = &mesh(&model, "EyeR").keyforms;
    assert_eq!(
        right.mirror_source,
        Some(ugoe_model::MirrorSource {
            id: "EyeL".into(),
            axis: None,
            tolerance: None,
        })
    );
    assert_eq!(right_origin(&model, 0.0, 0.0), Authored);
    assert_eq!(right_origin(&model, 30.0, 30.0), Additive);
    for (x, y) in [(-30.0, 0.0), (30.0, 0.0), (0.0, 30.0), (-30.0, -30.0)] {
        assert_eq!(right_origin(&model, x, y), Mirror, "({x}, {y})");
    }
    // 右目の AngleX = 30 は左目 (x +0.02) の反転で x −0.02、AngleY = 30 は y +0.03。
    let base = right_cell(&model, 0.0, 0.0).clone();
    assert_shift(right_cell(&model, 30.0, 30.0), &base, -0.02, 0.03);
}

#[test]
fn mirrored_cells_survive_edits_of_the_target_grid() {
    let model = mirrored_model();
    let base = right_cell(&model, 0.0, 0.0).clone();
    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "EyeR".into(),
            cell: xy(0.0, 0.0),
            keyform: shifted(&base, 0.0, 0.1),
        },
    );
    // ミラーのセルは加算合成で上書きしない (ミラー元の左目は変わっていない)。
    for (x, y) in [(-30.0, 0.0), (30.0, 0.0), (0.0, 30.0), (-30.0, -30.0)] {
        assert_eq!(
            right_cell(&after, x, y),
            right_cell(&model, x, y),
            "({x}, {y})"
        );
        assert_eq!(right_origin(&after, x, y), Mirror);
    }
    // 加算合成のセルは作り直す: 新しい基準 + (ミラーのセル − 新しい基準) の和。
    assert_eq!(right_origin(&after, 30.0, 30.0), Additive);
    assert_shift(right_cell(&after, 30.0, 30.0), &base, -0.02, 0.03 - 0.1);
}

#[test]
fn editing_the_source_remirrors_its_targets() {
    let model = mirrored_model();
    let left = base(&model);
    let right = right_cell(&model, 0.0, 0.0).clone();
    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "EyeL".into(),
            cell: xy(-30.0, 0.0),
            keyform: shifted(&left, -0.2, 0.0),
        },
    );
    // 左目の加算合成のセルを作り直し、右目のミラーのセルをミラーし直す (同じ Undo 単位)。
    assert_shift(eye_cell(&after, -30.0, -30.0), &left, -0.2, -0.01);
    assert_shift(right_cell(&after, -30.0, 0.0), &right, 0.2, 0.0);
    assert_shift(right_cell(&after, -30.0, -30.0), &right, 0.2, -0.01);
    assert_eq!(right_origin(&after, -30.0, -30.0), Mirror);
    // 右目の加算合成のセルの元 (AngleX = 30、AngleY = 30) は変わらない。
    assert_shift(right_cell(&after, 30.0, 30.0), &right, -0.02, 0.03);
}

#[test]
fn inference_keeps_cells_of_other_methods_unless_asked() {
    let model = mirrored_model();
    let infer = |overwrite_derived| Command::InferKeyforms {
        target: target("EyeR"),
        method: InferMethod::Additive,
        overwrite_derived,
    };
    let after = applied(&model, infer(false));
    assert_eq!(after, model);
    let after = applied(&model, infer(true));
    // 基準だけが `authored` なので、加算合成ではすべてのセルが基準と同じになる。
    let base = right_cell(&model, 0.0, 0.0).clone();
    for (x, y) in [(-30.0, 0.0), (30.0, 30.0)] {
        assert_eq!(right_origin(&after, x, y), Additive);
        assert_shift(right_cell(&after, x, y), &base, 0.0, 0.0);
    }
}

#[test]
fn legacy_derived_cells_use_the_grid_method() {
    // 方法を記録しない版のミラー (由来は `derived`、ミラー元の記録なし) は、格子の方法で作り直す。
    let mut model = mirror_model();
    ugoe_rigging::mirror_keyforms(
        &mut model,
        &(&target("EyeL")).into(),
        &(&target("EyeR")).into(),
        &MirrorOptions::default(),
    )
    .unwrap();
    let right = &mut mesh_mut(&mut model, "EyeR").keyforms;
    right.mirror_source = None;
    for origin in &mut right.origins {
        if *origin == Mirror {
            *origin = Derived;
        }
    }
    let base = right_cell(&model, 0.0, 0.0).clone();
    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "EyeR".into(),
            cell: xy(0.0, 0.0),
            keyform: shifted(&base, 0.0, 0.1),
        },
    );
    assert_eq!(right_origin(&after, -30.0, 0.0), Additive);
    assert_shift(right_cell(&after, -30.0, 0.0), &base, 0.0, 0.1);
}

/// `EyeL` と `EyeR` を AngleX (キー -30 0 30) の 1 軸の格子にし、互いにミラー元にしたモデル。
/// 左目は AngleX = -30 を右目から、右目は AngleX = 30 を左目からミラーする。
fn cycle_model() -> Model {
    let mut model = fixtures::keyform_inference();
    let left = mesh(&model, "EyeL").keyforms.keyforms[0].clone();
    let right = mesh(&model, "EyeR").keyforms.keyforms[0].clone();
    let axis = || vec![binding("AngleX", &KEYS)];
    let eye = mesh_mut(&mut model, "EyeL");
    eye.keyforms = KeyformGrid::new(
        axis(),
        vec![left.clone(), left.clone(), shifted(&left, 0.02, 0.0)],
    );
    eye.keyforms.origins = vec![Derived, Authored, Authored];
    let eye = mesh_mut(&mut model, "EyeR");
    eye.keyforms = KeyformGrid::new(
        axis(),
        vec![shifted(&right, 0.04, 0.0), right.clone(), right.clone()],
    );
    eye.keyforms.origins = vec![Authored, Authored, Derived];
    let mut doc = Document::new(model);
    for (source, to) in [("EyeR", "EyeL"), ("EyeL", "EyeR")] {
        doc.apply(Command::MirrorKeyforms {
            source: target(source),
            target: target(to),
            axis: None,
            tolerance: None,
            overwrite_authored: false,
        })
        .unwrap();
    }
    doc.model().clone()
}

#[test]
fn mirror_cycles_between_two_grids() {
    let model = cycle_model();
    let cells = |model: &Model, id: &str| mesh(model, id).keyforms.keyforms.clone();
    let origins = |model: &Model, id: &str| mesh(model, id).keyforms.origins.clone();
    assert_eq!(origins(&model, "EyeL"), vec![Mirror, Authored, Authored]);
    assert_eq!(origins(&model, "EyeR"), vec![Authored, Authored, Mirror]);
    let (left, right) = (cells(&model, "EyeL"), cells(&model, "EyeR"));
    assert_shift(&left[0], &left[1], -0.04, 0.0);
    assert_shift(&right[2], &right[1], -0.02, 0.0);

    // 左目の AngleX = 30 を直すと右目の AngleX = 30 をミラーし直し、右目の変化は左目へ戻る
    // (左目の AngleX = -30 の元は変わらないので、そこで止まる)。
    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "EyeL".into(),
            cell: cell(&[("AngleX", 30.0)]),
            keyform: shifted(&left[1], 0.1, 0.0),
        },
    );
    assert_shift(&cells(&after, "EyeR")[2], &right[1], -0.1, 0.0);
    assert_eq!(cells(&after, "EyeL")[0], left[0]);

    // 右目の AngleX = -30 を直すと、左目の AngleX = -30 をミラーし直す。
    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "EyeR".into(),
            cell: cell(&[("AngleX", -30.0)]),
            keyform: shifted(&right[1], 0.3, 0.0),
        },
    );
    assert_shift(&cells(&after, "EyeL")[0], &left[1], -0.3, 0.0);
    assert_eq!(cells(&after, "EyeR")[2], right[2]);

    // 両方がミラーのセルで互いを元にしていても (値の元がない循環)、作り直しは止まる。
    let mut degenerate = model.clone();
    mesh_mut(&mut degenerate, "EyeR").keyforms.origins[0] = Mirror;
    let after = applied(
        &degenerate,
        Command::SetMeshKeyform {
            id: "EyeL".into(),
            cell: cell(&[("AngleX", 0.0)]),
            keyform: shifted(&left[1], 0.0, 0.001),
        },
    );
    assert_eq!(origins(&after, "EyeR"), vec![Mirror, Authored, Mirror]);
}

#[test]
fn self_mirrored_cells_follow_edits_of_the_same_grid() {
    let mut model = fixtures::keyform_inference();
    model.parameters[0].mirror = Some("AngleX".into());
    let base = mesh(&model, "Nose").keyforms.keyforms[0].clone();
    let nose = mesh_mut(&mut model, "Nose");
    nose.keyforms = KeyformGrid::new(
        vec![binding("AngleX", &KEYS)],
        vec![base.clone(), base.clone(), shifted(&base, 0.02, 0.0)],
    );
    nose.keyforms.origins = vec![Derived, Authored, Authored];
    let mut doc = Document::new(model);
    doc.apply(Command::MirrorKeyforms {
        source: target("Nose"),
        target: target("Nose"),
        axis: None,
        tolerance: None,
        overwrite_authored: false,
    })
    .unwrap();
    let model = doc.model().clone();
    let nose = &mesh(&model, "Nose").keyforms;
    assert_eq!(nose.origins[0], Mirror);
    assert_shift(&nose.keyforms[0], &base, -0.02, 0.0);

    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "Nose".into(),
            cell: cell(&[("AngleX", 30.0)]),
            keyform: shifted(&base, 0.05, 0.0),
        },
    );
    assert_shift(
        &mesh(&after, "Nose").keyforms.keyforms[0],
        &base,
        -0.05,
        0.0,
    );
}

#[test]
fn removing_a_mirror_source_freezes_the_mirrored_cells() {
    let model = mirrored_model();
    let after = applied(&model, Command::RemoveMesh { id: "EyeL".into() });
    let right = &mesh(&after, "EyeR").keyforms;
    assert_eq!(right.mirror_source, None);
    assert!(!right.origins.contains(&Mirror));
    assert_eq!(right.keyforms, mesh(&model, "EyeR").keyforms.keyforms);
    assert_eq!(right_origin(&after, -30.0, 0.0), Authored);
    assert_eq!(right_origin(&after, 30.0, 30.0), Additive);
}

#[test]
fn dangling_mirror_sources_are_rejected() {
    let model = mirrored_model();
    let mut keyforms = mesh(&model, "EyeR").keyforms.clone();
    if let Some(source) = &mut keyforms.mirror_source {
        source.id = "Missing".into();
    }
    let error = rejected(
        &model,
        Command::SetMeshKeyforms {
            id: "EyeR".into(),
            keyforms,
        },
    );
    let EditError::Invalid(errors) = error else {
        panic!("expected a validation error: {error:?}");
    };
    assert!(errors.iter().any(|e| matches!(
        e,
        ugoe_model::validate::ValidationError::UnknownReference { field, target, .. }
            if *field == "keyforms.mirror_source" && target == "Missing"
    )));
}

// ---- 自動のミラーのやり直しの失敗 (RederiveWarning) ----

/// `doc` に `command` を適用し、警告を返す (コマンドは拒否されない)。
fn warnings_of(model: &Model, command: Command) -> (Model, Vec<RederiveWarning>) {
    let mut doc = Document::new(model.clone());
    let report = doc.apply_with_report(command).unwrap();
    (doc.model().clone(), report.warnings)
}

#[test]
fn successful_rederivation_reports_no_warnings() {
    let model = mirrored_model();
    let left = base(&model);
    let (_, warnings) = warnings_of(
        &model,
        Command::SetMeshKeyform {
            id: "EyeL".into(),
            cell: xy(-30.0, 0.0),
            keyform: shifted(&left, -0.2, 0.0),
        },
    );
    assert_eq!(warnings, vec![]);
}

#[test]
fn failed_remirror_keeps_the_cells_and_warns() {
    let model = mirrored_model();
    // 左目の頂点 0 を大きく動かすと、右目の頂点 0 に対応する左目の頂点がなくなる。
    let command = Command::MoveMeshVertices {
        id: "EyeL".into(),
        moves: vec![crate::VertexMove {
            index: 0,
            delta: Vec2::new(0.0, 100.0),
        }],
    };
    let (after, warnings) = warnings_of(&model, command.clone());
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    let warning = &warnings[0];
    assert_eq!(warning.target, target("EyeR"));
    assert_eq!(warning.source, target("EyeL"));
    assert!(
        warning.message.contains("no mirrored source vertex"),
        "{}",
        warning.message
    );
    // 右目のミラーのセルは前の値のまま。
    assert_eq!(mesh(&after, "EyeR").keyforms, mesh(&model, "EyeR").keyforms);
    // `apply` は同じ結果になり、警告を捨てるだけ。
    assert_eq!(applied(&model, command), after);
}

#[test]
fn unpairing_the_mirror_source_warns() {
    let model = mirrored_model();
    let (after, warnings) = warnings_of(
        &model,
        Command::SetMirrorPair {
            target: target("EyeL"),
            mirror: None,
            axis: None,
        },
    );
    assert_eq!(mesh(&after, "EyeR").mirror, None);
    assert_eq!(
        warnings
            .iter()
            .map(|w| (&w.target, &w.source))
            .collect::<Vec<_>>(),
        vec![(&target("EyeR"), &target("EyeL"))]
    );
    assert_eq!(mesh(&after, "EyeR").keyforms, mesh(&model, "EyeR").keyforms);
}

// ---- SetKeyformInference ----

fn face_grid(model: &Model) -> &KeyformGrid<ugoe_model::deformer::WarpKeyform> {
    match &model.deformers[0].kind {
        ugoe_model::DeformerKind::Warp(warp) => &warp.keyforms,
        _ => panic!("Face is a warp deformer"),
    }
}

fn face_rotation(model: &Model) -> ugoe_model::DepthRotation {
    match &face_grid(model).inference {
        Some(KeyformInference::DepthRotation(rotation)) => rotation.clone(),
        other => panic!("Face must use depth rotation: {other:?}"),
    }
}

#[test]
fn set_keyform_inference_rederives_with_the_new_settings() {
    let model = fixtures::keyform_inference();
    let mut rotation = face_rotation(&model);
    rotation.angle_scale = 0.5;
    let inference = Some(KeyformInference::DepthRotation(rotation));
    let mut expected = model.clone();
    if let ugoe_model::DeformerKind::Warp(warp) = &mut expected.deformers[0].kind {
        warp.keyforms.inference = inference.clone();
    }
    ugoe_rigging::infer_depth_rotation(&mut expected, &(&face()).into(), InferScope::Own).unwrap();
    let after = applied(
        &model,
        Command::SetKeyformInference {
            target: face(),
            inference,
        },
    );
    assert_eq!(after, expected);
    assert_eq!(face_grid(&after).origin(8), DepthRotation);
}

#[test]
fn set_keyform_inference_to_additive_rederives_legacy_cells() {
    let model = fixtures::keyform_inference();
    let mut expected = model.clone();
    if let ugoe_model::DeformerKind::Warp(warp) = &mut expected.deformers[0].kind {
        warp.keyforms.inference = None;
    }
    ugoe_rigging::infer_additive_target(&mut expected, &(&face()).into(), InferScope::Own).unwrap();
    let after = applied(
        &model,
        Command::SetKeyformInference {
            target: face(),
            inference: None,
        },
    );
    assert_eq!(after, expected);
    assert_eq!(face_grid(&after).inference, None);
}

#[test]
fn set_keyform_inference_rejects_invalid_settings() {
    let model = fixtures::keyform_inference();
    assert_eq!(
        rejected(
            &model,
            Command::SetKeyformInference {
                target: face(),
                inference: Some(KeyformInference::Unknown),
            },
        ),
        EditError::UnsupportedInference {
            target: face().to_string()
        }
    );
    // yaw のパラメータは格子の軸でなければならない (検証で拒否する)。
    let mut rotation = face_rotation(&model);
    rotation.yaw = Some("EyeLOpen".into());
    let error = rejected(
        &model,
        Command::SetKeyformInference {
            target: target("EyeL"),
            inference: Some(KeyformInference::DepthRotation(rotation)),
        },
    );
    assert!(matches!(error, EditError::Invalid(_)), "{error:?}");
    assert!(matches!(
        rejected(
            &model,
            Command::SetKeyformInference {
                target: target("Missing"),
                inference: None,
            },
        ),
        EditError::NotFound { kind: "mesh", .. }
    ));
}

// ---- SetMirrorPair ----

#[test]
fn set_mirror_pair_links_both_sides() {
    let model = fixtures::keyform_inference();
    assert_eq!(mesh(&model, "EyeL").mirror, Some("EyeR".into()));
    let after = applied(
        &model,
        Command::SetMirrorPair {
            target: target("Nose"),
            mirror: Some("EyeL".into()),
            axis: Some(0.25),
        },
    );
    assert_eq!(mesh(&after, "Nose").mirror, Some("EyeL".into()));
    assert_eq!(mesh(&after, "Nose").mirror_axis, Some(0.25));
    assert_eq!(mesh(&after, "EyeL").mirror, Some("Nose".into()));
    // EyeL の前の相手は対を外す。
    assert_eq!(mesh(&after, "EyeR").mirror, None);
    assert_eq!(
        mesh(&after, "EyeR").mirror_axis,
        mesh(&model, "EyeR").mirror_axis
    );

    // 自分自身は「対にしない」と同じ扱いで、相手の対も外す。
    let after = applied(
        &model,
        Command::SetMirrorPair {
            target: target("EyeL"),
            mirror: Some("EyeL".into()),
            axis: None,
        },
    );
    assert_eq!(mesh(&after, "EyeL").mirror, Some("EyeL".into()));
    assert_eq!(mesh(&after, "EyeL").mirror_axis, None);
    assert_eq!(mesh(&after, "EyeR").mirror, None);
}

#[test]
fn set_mirror_pair_rejects_invalid_targets() {
    let model = fixtures::keyform_inference();
    let pair = |target, mirror: Option<&str>, axis| Command::SetMirrorPair {
        target,
        mirror: mirror.map(Into::into),
        axis,
    };
    assert!(matches!(
        rejected(&model, pair(target("EyeL"), Some("Missing"), None)),
        EditError::NotFound { kind: "mesh", .. }
    ));
    assert!(matches!(
        rejected(&model, pair(target("Missing"), None, None)),
        EditError::NotFound { kind: "mesh", .. }
    ));
    // メッシュの ID をデフォーマとして指定しても見つからない。
    assert!(matches!(
        rejected(&model, pair(face(), Some("EyeL"), None)),
        EditError::NotFound {
            kind: "deformer",
            ..
        }
    ));
    let part = KeyformTarget::Part(model.parts[0].id.clone());
    assert_eq!(
        rejected(&model, pair(part.clone(), None, None)),
        EditError::MirrorPairNotSupported {
            target: part.to_string()
        }
    );
    assert!(matches!(
        rejected(&model, pair(target("EyeL"), None, Some(f32::NAN))),
        EditError::Invalid(_)
    ));
}

// ---- 見た目を保つ付け替えとミラー (T0607 × T0614) ----

fn reparent_to_root(id: &str) -> Command {
    Command::ReparentPreservingLook {
        element: crate::ChildRef::Mesh(id.into()),
        new_parent: None,
    }
}

/// `keyform_inference` の `Face` (64..192 の正方形のワープ) の UV → キャンバス座標。
fn face_to_canvas(p: Vec2) -> Vec2 {
    Vec2::splat(64.0) + p * 128.0
}

#[track_caller]
fn assert_positions(actual: &MeshKeyform, expected: impl IntoIterator<Item = Vec2>) {
    for (p, q) in actual.positions.iter().zip(expected) {
        assert!(p.abs_diff_eq(q, 1e-3), "{p} != {q}");
    }
}

#[test]
fn reparenting_a_mirror_target_keeps_its_cells_and_source() {
    // 右目 (ミラー先) だけをワープの外へ出す。検証を通り、ミラー元の記録と由来はそのまま、
    // ミラーのセルは見た目を保って写し直される (ミラー元が変わっていないので、ミラーし直さない)。
    let model = mirrored_model();
    let mut doc = Document::new(model.clone());
    let report = doc.apply_with_report(reparent_to_root("EyeR")).unwrap();
    assert_eq!(report.warnings, vec![]);
    let after = applied(&model, reparent_to_root("EyeR"));
    let (before, right) = (
        &mesh(&model, "EyeR").keyforms,
        &mesh(&after, "EyeR").keyforms,
    );
    assert_eq!(right.mirror_source, before.mirror_source);
    assert_eq!(right.origins, before.origins);
    for (b, a) in before.keyforms.iter().zip(&right.keyforms) {
        assert_positions(a, b.positions.iter().map(|&p| face_to_canvas(p)));
    }

    // 片側だけ付け替えると対の親が異なる。T0615 の時点ではミラー元の座標系で反転した値をそのまま
    // 書いたのでミラーし直せず警告を返したが、T0618 からはキャンバス座標を経由して反転するので、
    // ミラー元を編集すると警告なしにミラーし直される。結果は、付け替えずに編集したものを
    // キャンバス座標へ写したものと同じ (`Face` はどのセルでも同じ形なので、姿勢によらない)。
    let left = base(&after);
    let edit = Command::SetMeshKeyform {
        id: "EyeL".into(),
        cell: xy(-30.0, 0.0),
        keyform: shifted(&left, -0.2, 0.0),
    };
    let mut doc = Document::new(after.clone());
    let report = doc.apply_with_report(edit.clone()).unwrap();
    assert_eq!(report.warnings, vec![]);
    let mut unmoved = Document::new(model.clone());
    unmoved.apply(edit.clone()).unwrap();
    for (x, y) in [(-30.0, 0.0), (-30.0, -30.0), (30.0, 0.0)] {
        assert_eq!(right_origin(doc.model(), x, y), Mirror);
        assert_positions(
            right_cell(doc.model(), x, y),
            right_cell(unmoved.model(), x, y)
                .positions
                .iter()
                .map(|&p| face_to_canvas(p)),
        );
    }

    // 左目も付け替えて座標系をそろえると、ミラー元の編集はまたミラーし直される。
    // 結果は、付け替えずに編集したものをキャンバス座標へ写したものと同じ。
    let both = applied(&after, reparent_to_root("EyeL"));
    let left = base(&both);
    let mut doc = Document::new(both);
    let report = doc
        .apply_with_report(Command::SetMeshKeyform {
            id: "EyeL".into(),
            cell: xy(-30.0, 0.0),
            keyform: MeshKeyform {
                positions: base(&model)
                    .positions
                    .iter()
                    .map(|&p| face_to_canvas(p + Vec2::new(-0.2, 0.0)))
                    .collect(),
                ..left
            },
        })
        .unwrap();
    assert_eq!(report.warnings, vec![]);
    for (x, y) in [(-30.0, 0.0), (-30.0, -30.0), (30.0, 0.0)] {
        assert_eq!(right_origin(doc.model(), x, y), Mirror);
        assert_positions(
            right_cell(doc.model(), x, y),
            right_cell(unmoved.model(), x, y)
                .positions
                .iter()
                .map(|&p| face_to_canvas(p)),
        );
    }
}

#[test]
fn reparenting_a_self_mirrored_element_remirrors_in_the_new_space() {
    // 左右対称な 1 つのメッシュ (`Nose`、ミラー元が自分自身) の AngleX = 30 をミラーで作り、
    // ワープの外へ出す。付け替えは格子を変えるので、ミラーのセルは新しい座標系でミラーし直され、
    // 写し直した値と一致する (対称軸の既定値はワープの 0.5 からキャンバスの原点 128 に変わるが、
    // どちらも同じ縦の直線)。
    let mut model = fixtures::keyform_inference();
    // AngleX を自分自身と対にする (AngleX = 30 のセルは AngleX = -30 のセルの反転)。
    let angle_x = model
        .parameters
        .iter_mut()
        .find(|p| p.id.as_str() == "AngleX");
    angle_x.unwrap().mirror = Some("AngleX".into());
    let nose = mesh_mut(&mut model, "Nose");
    let base = nose.keyforms.keyforms[0].clone();
    nose.keyforms = KeyformGrid::new(
        vec![binding("AngleX", &KEYS)],
        vec![shifted(&base, -0.05, 0.0), base.clone(), base],
    );
    nose.keyforms.origins = vec![Authored, Authored, Derived];
    let mut doc = Document::new(model);
    doc.apply(Command::MirrorKeyforms {
        source: target("Nose"),
        target: target("Nose"),
        axis: None,
        tolerance: None,
        overwrite_authored: false,
    })
    .unwrap();
    let model = doc.model().clone();
    let before = &mesh(&model, "Nose").keyforms;
    assert_eq!(before.origin(2), Mirror);
    assert_shift(&before.keyforms[2], &before.keyforms[1], 0.05, 0.0);

    let mut doc = Document::new(model.clone());
    let report = doc.apply_with_report(reparent_to_root("Nose")).unwrap();
    assert_eq!(report.warnings, vec![]);
    let after = applied(&model, reparent_to_root("Nose"));
    let nose = &mesh(&after, "Nose").keyforms;
    assert_eq!(nose.mirror_source, before.mirror_source);
    assert_eq!(nose.origins, before.origins);
    for (b, a) in before.keyforms.iter().zip(&nose.keyforms) {
        assert_positions(a, b.positions.iter().map(|&p| face_to_canvas(p)));
    }
}

// ---- 親が異なる左右の対 (T0618) ----

/// `mirrored_model` の右目のキャンバス座標 (既定値の姿勢)。
fn right_canvas(model: &Model, x: f32, y: f32) -> Vec<Vec2> {
    crate::space::local_to_canvas(
        model,
        &target("EyeR"),
        &Default::default(),
        &right_cell(model, x, y).positions,
    )
    .unwrap()
}

#[test]
fn wrapping_one_side_in_a_new_warp_keeps_remirroring() {
    // 右目だけを新しいワープで包む (親が `Face` と `New` に分かれる)。対は対応したままで警告なし。
    let model = mirrored_model();
    let wrap = Command::CreateWarpDeformer {
        id: Some("New".into()),
        name: "New".into(),
        children: vec![crate::ChildRef::Mesh("EyeR".into())],
        parts: Vec::new(),
        rows: 2,
        cols: 2,
        parent: None,
    };
    let mut doc = Document::new(model.clone());
    assert_eq!(doc.apply_with_report(wrap).unwrap().warnings, vec![]);
    assert_eq!(mesh(doc.model(), "EyeR").parent, Some("New".into()));

    // ミラー元を編集すると、キャンバス座標を経由してミラーし直される。見た目は、付け替えずに
    // 編集したときと同じ。
    let edit = Command::SetMeshKeyform {
        id: "EyeL".into(),
        cell: xy(30.0, 0.0),
        keyform: shifted(&base(&model), 0.07, -0.02),
    };
    assert_eq!(
        doc.apply_with_report(edit.clone()).unwrap().warnings,
        vec![]
    );
    let unmoved = applied(&model, edit);
    for (x, y) in [(-30.0, 0.0), (-30.0, -30.0), (30.0, 0.0), (30.0, 30.0)] {
        let expected: Vec<Vec2> = right_cell(&unmoved, x, y)
            .positions
            .iter()
            .map(|&p| face_to_canvas(p))
            .collect();
        for (p, q) in right_canvas(doc.model(), x, y).iter().zip(&expected) {
            assert!(p.abs_diff_eq(*q, 1e-2), "({x}, {y}): {p} != {q}");
        }
    }
}

/// `mirrored_model` の `Face` を x に −20 ずらし、目の `mirror_axis` を外したモデル。
/// 左右の目は `Face` の UV 空間の u = 0.5 (キャンバスの x = 108) で対称で、キャンバスの原点
/// (x = 128) では対称でない。
fn off_center_model() -> Model {
    let mut model = mirrored_model();
    for id in ["EyeL", "EyeR"] {
        mesh_mut(&mut model, id).mirror_axis = None;
    }
    let face = model
        .deformers
        .iter_mut()
        .find(|d| d.id == "Face".into())
        .unwrap();
    if let ugoe_model::deformer::DeformerKind::Warp(w) = &mut face.kind {
        for kf in &mut w.keyforms.keyforms {
            kf.points.iter_mut().for_each(|p| p.x -= 20.0);
        }
    }
    model
}

#[test]
fn reparenting_breaks_an_off_center_pair_and_warns() {
    // 親が同じなら対称軸の既定値は親の UV 空間の 0.5。右目を外に出すと親が異なり、既定値は
    // キャンバスの原点になるので対応しなくなる。付け替えは見た目を保って行い、警告を返す。
    let model = off_center_model();
    let mut doc = Document::new(model.clone());
    let report = doc.apply_with_report(reparent_to_root("EyeR")).unwrap();
    assert_eq!(
        report
            .warnings
            .iter()
            .map(|w| (&w.target, &w.source))
            .collect::<Vec<_>>(),
        vec![(&target("EyeR"), &target("EyeL"))]
    );
    assert!(
        report.warnings[0]
            .message
            .contains("no mirrored source vertex"),
        "{}",
        report.warnings[0].message
    );
    // `apply` は同じ結果になる (警告を捨てるだけ)。
    assert_eq!(&applied(&model, reparent_to_root("EyeR")), doc.model());

    // 元の対称軸を `Face` の UV 空間で指定すると (キャンバスへ写すと x = 108)、また対応する。
    let report = doc
        .apply_with_report(Command::SetMirrorPair {
            target: target("EyeL"),
            mirror: Some("EyeR".into()),
            axis: Some(0.5),
        })
        .unwrap();
    assert_eq!(report.warnings, vec![]);
    let edit = Command::SetMeshKeyform {
        id: "EyeL".into(),
        cell: xy(30.0, 0.0),
        keyform: shifted(&base(&model), 0.07, 0.0),
    };
    assert_eq!(doc.apply_with_report(edit).unwrap().warnings, vec![]);
    // 右目の AngleX = 30 は、左目の AngleX = 30 (u +0.07) をキャンバスの x = 108 で反転したもの
    // (このモデルの AngleX は対にしていない)。
    let left: Vec<Vec2> = shifted(&base(&model), 0.07, 0.0)
        .positions
        .iter()
        .map(|&p| Vec2::new(216.0 - (44.0 + p.x * 128.0), 64.0 + p.y * 128.0))
        .collect();
    let mut right = right_canvas(doc.model(), 30.0, 0.0);
    right.sort_by(|a, b| (a.x, a.y).partial_cmp(&(b.x, b.y)).unwrap());
    let mut left = left;
    left.sort_by(|a, b| (a.x, a.y).partial_cmp(&(b.x, b.y)).unwrap());
    for (p, q) in right.iter().zip(&left) {
        assert!(p.abs_diff_eq(*q, 1e-2), "{p} != {q}");
    }
}

#[test]
fn reparenting_the_source_warns_once() {
    // ミラー元を付け替えると、依存する格子をミラーし直す (失敗すれば警告)。付け替えの検査は
    // 同じ組の警告を重ねない。
    let model = off_center_model();
    let (after, warnings) = warnings_of(&model, reparent_to_root("EyeL"));
    assert_eq!(
        warnings
            .iter()
            .map(|w| (&w.target, &w.source))
            .collect::<Vec<_>>(),
        vec![(&target("EyeR"), &target("EyeL"))]
    );
    assert_eq!(mesh(&after, "EyeR").keyforms, mesh(&model, "EyeR").keyforms);
}

#[test]
fn reparenting_an_unpaired_or_already_broken_pair_does_not_warn() {
    // 対のない要素 (`Nose`) の付け替えは検査しない。
    let model = off_center_model();
    let (_, warnings) = warnings_of(&model, reparent_to_root("Nose"));
    assert_eq!(warnings, vec![]);
    // 付け替えの前からミラーできない対は、付け替えでは警告しない (壊れたときに警告済み)。
    let mut broken = model.clone();
    mesh_mut(&mut broken, "EyeR").keyforms.keyforms[4]
        .positions
        .iter_mut()
        .for_each(|p| p.y += 0.2);
    let (_, warnings) = warnings_of(&broken, reparent_to_root("EyeR"));
    assert_eq!(warnings, vec![]);
}
