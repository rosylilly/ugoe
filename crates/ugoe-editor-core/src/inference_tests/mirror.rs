//! ミラーのコマンド (`MirrorKeyforms`・`SetMirrorPair`) のテスト。

use super::*;

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

// ミラーは通常の格子だけを書き換え、ブレンドシェイプ格子 (と、そのミラー元の記録) は変えない
// (`docs/spec/keyform-inference.md`)。
#[test]
fn mirror_keeps_blend_shape_grids() {
    let mut model = mirror_model();
    model.parameters.push(ugoe_model::Parameter {
        id: "Smile".into(),
        name: "Smile".into(),
        min: 0.0,
        default: 0.0,
        max: 30.0,
        kind: ugoe_model::parameter::ParameterKind::BlendShape,
        mirror: None,
        group: None,
        internal: false,
    });
    for (id, dx) in [("EyeL", -0.01), ("EyeR", 0.02)] {
        let eye = mesh_mut(&mut model, id);
        let base = eye.keyforms.keyforms[0].clone();
        let mut shape = KeyformGrid::new(
            vec![binding("Smile", &[0.0, 30.0])],
            vec![base.clone(), shifted(&base, dx, 0.0)],
        );
        shape.mirror_source = (id == "EyeR").then(|| ugoe_model::MirrorSource {
            id: "EyeL".into(),
            axis: None,
            tolerance: None,
        });
        eye.keyforms.blend_shapes = vec![shape];
    }
    let after = applied(&model, mirror_eyes(false));
    for id in ["EyeL", "EyeR"] {
        assert_eq!(
            mesh(&after, id).keyforms.blend_shapes,
            mesh(&model, id).keyforms.blend_shapes,
            "{id}"
        );
    }
    assert_ne!(
        mesh(&after, "EyeR").keyforms.keyforms,
        mesh(&model, "EyeR").keyforms.keyforms
    );
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
        .find(|p| p.id == "AngleX")
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
