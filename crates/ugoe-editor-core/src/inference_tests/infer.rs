//! 推定のコマンド (`InferKeyforms`・`MarkKeyformAuthored` / `MarkKeyformDerived`・`SetKeyformInference`) のテスト。

use super::*;

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

    // 検証の前の格子 (キーが昇順でない・同じパラメータの軸が 2 本) も推定のエラーで拒否する。
    let mut unsorted = model.clone();
    mesh_mut(&mut unsorted, "EyeL").keyforms.bindings[0].keys = vec![30.0, 0.0, -30.0];
    assert_eq!(
        rejected(
            &unsorted,
            Command::InferKeyforms {
                target: target("EyeL"),
                method: InferMethod::Additive,
                overwrite_derived: false,
            },
        ),
        inference_error(
            &target("EyeL"),
            InferError::KeysNotAscending {
                parameter: "AngleX".into(),
                keys: vec![30.0, 0.0, -30.0],
            }
        )
    );
    let mut duplicate = model.clone();
    mesh_mut(&mut duplicate, "EyeL").keyforms.bindings[1].parameter = "AngleX".into();
    assert_eq!(
        rejected(
            &duplicate,
            Command::InferKeyforms {
                target: target("EyeL"),
                method: InferMethod::Additive,
                overwrite_derived: false,
            },
        ),
        inference_error(
            &target("EyeL"),
            InferError::DuplicateAxis {
                parameter: "AngleX".into(),
            }
        )
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

// ---- SetKeyformInference ----

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
