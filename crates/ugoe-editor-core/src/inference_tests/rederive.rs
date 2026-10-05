//! `derived` のセルの自動の作り直し (セルの由来ごとの作り直しと、失敗の警告 `RederiveWarning`) のテスト。

use super::*;

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
fn removing_a_glue_forgets_it_as_a_mirror_source() {
    // `Pull` の格子のミラー元が `Seam`。`Seam` を消すと、メッシュの削除と同じくミラー元の記録を外す。
    let mut model = fixtures::glued_quads();
    let pull = model.glues.iter_mut().find(|g| g.id == "Pull").unwrap();
    pull.keyforms.mirror_source = Some(ugoe_model::MirrorSource {
        id: "Seam".into(),
        axis: None,
        tolerance: None,
    });
    let after = applied(&model, Command::RemoveGlue { id: "Seam".into() });
    let pull = after.glues.iter().find(|g| g.id == "Pull").unwrap();
    assert_eq!(pull.keyforms.mirror_source, None);
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
    // 画面はコードと引数で訳す (ADR 0029)。
    assert_eq!(warning.code, "inference.mirror_failed");
    assert_eq!(
        warning.args,
        crate::message_args! {
            "target" => "mesh `EyeR`",
            "source" => "mesh `EyeL`",
            "detail" => &warning.message,
        }
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
