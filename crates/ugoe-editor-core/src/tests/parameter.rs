//! パラメータのコマンドのテスト。

use super::*;

#[test]
fn add_parameter() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::AddParameter {
            parameter: new_parameter("Y", -1.0, 0.0, 1.0),
        },
    );
    assert_eq!(after.parameters.len(), 2);
    assert_eq!(
        rejected(
            &model,
            Command::AddParameter {
                parameter: new_parameter("X", -1.0, 0.0, 1.0),
            },
        ),
        EditError::DuplicateId {
            kind: "parameter",
            id: "X".into(),
        }
    );
    rejected_invalid(
        &model,
        Command::AddParameter {
            parameter: new_parameter("Y", 1.0, 0.0, -1.0),
        },
        |e| matches!(e, ValidationError::ParameterRange { .. }),
    );
}

#[test]
fn remove_parameter_collapses_axis_at_default_key() {
    let model = fixtures::single_quad();
    let after = applied(&model, Command::RemoveParameter { id: "X".into() });
    assert!(after.parameters.is_empty());
    let quad = &after.meshes[0].keyforms;
    assert!(quad.bindings.is_empty());
    // 既定値 0 のキー (中央) の断面が残る。
    assert_eq!(
        quad.keyforms,
        vec![model.meshes[0].keyforms.keyforms[1].clone()]
    );
}

#[test]
fn remove_parameter_collapses_axis_at_nearest_key() {
    let mut model = fixtures::rotation_chain();
    // `Arm` の軸 `Angle` のキーは -30 と 30。既定値 10 はキーでないので、近い 30 の断面が残る。
    model.parameters[0].default = 10.0;
    let after = applied(&model, Command::RemoveParameter { id: "Angle".into() });
    let DeformerKind::Rotation(arm) = &after.deformers[0].kind else {
        panic!("Arm is a rotation deformer");
    };
    let DeformerKind::Rotation(before) = &model.deformers[0].kind else {
        panic!("Arm is a rotation deformer");
    };
    assert!(arm.keyforms.bindings.is_empty());
    assert_eq!(arm.keyforms.keyforms, vec![before.keyforms.keyforms[1]]);
}

#[test]
fn remove_parameter_collapses_glue_and_part_grids() {
    let model = fixtures::glued_quads();
    let after = applied(&model, Command::RemoveParameter { id: "Glue".into() });
    assert!(after.glues[0].keyforms.bindings.is_empty());
    assert_eq!(after.glues[0].keyforms.keyforms[0].intensity, 1.0);

    let model = fixtures::part_opacity_hierarchy();
    let mut doc = Document::new(model.clone());
    for p in &model.parameters {
        doc.apply(Command::RemoveParameter { id: p.id.clone() })
            .unwrap();
    }
    assert!(
        doc.model()
            .parts
            .iter()
            .all(|p| p.keyforms.bindings.is_empty() && p.keyforms.keyforms.len() == 1)
    );
}

#[test]
fn remove_parameter_collapses_origins_and_inference_axes() {
    use ugoe_model::KeyformInference;
    use ugoe_model::KeyformOrigin::{Authored, Derived};
    let model = fixtures::keyform_inference();
    let face = |model: &Model| match &model.deformers[0].kind {
        DeformerKind::Warp(warp) => warp.keyforms.clone(),
        _ => panic!("Face is a warp deformer"),
    };
    let after = applied(
        &model,
        Command::RemoveParameter {
            id: "AngleX".into(),
        },
    );
    let grid = face(&after);
    assert_eq!(grid.bindings.len(), 1);
    // `AngleX = 0` の断面 (線形インデックス 1, 4, 7) が残る。中央だけが `authored`。
    assert_eq!(grid.origins, vec![Derived, Authored, Derived]);
    let Some(KeyformInference::DepthRotation(rotation)) = &grid.inference else {
        panic!("inference must stay depth rotation");
    };
    assert_eq!(rotation.yaw, None);
    assert_eq!(rotation.pitch, Some("AngleY".into()));

    let after = applied(
        &after,
        Command::RemoveParameter {
            id: "AngleY".into(),
        },
    );
    let grid = face(&after);
    assert!(grid.bindings.is_empty());
    assert_eq!(grid.origins, vec![Authored]);
    let Some(KeyformInference::DepthRotation(rotation)) = &grid.inference else {
        panic!("inference must stay depth rotation");
    };
    assert_eq!(
        (rotation.yaw.as_ref(), rotation.pitch.as_ref()),
        (None, None)
    );
}

#[test]
fn remove_parameter_drops_mirror_and_effect_references() {
    let mut model = fixtures::pose_and_effects();
    let right = model
        .parameters
        .iter_mut()
        .find(|p| p.id == "EyeROpen")
        .unwrap();
    right.mirror = Some("EyeLOpen".into());
    let after = applied(
        &model,
        Command::Batch {
            label: "remove".into(),
            commands: vec![
                Command::RemoveParameter {
                    id: "EyeLOpen".into(),
                },
                Command::RemoveParameter {
                    id: "Breath".into(),
                },
                Command::RemoveParameter {
                    id: "MouthOpenY".into(),
                },
            ],
        },
    );
    let right = after
        .parameters
        .iter()
        .find(|p| p.id == "EyeROpen")
        .unwrap();
    assert_eq!(right.mirror, None);
    assert_eq!(after.effects.eye_blink.parameters, vec!["EyeROpen"]);
    assert!(
        after
            .effects
            .breath
            .parameters
            .iter()
            .all(|b| b.parameter != "Breath")
    );
    assert_eq!(after.effects.breath.parameters.len(), 4);
    assert!(after.effects.lip_sync.parameters.is_empty());

    assert_eq!(
        rejected(&model, Command::RemoveParameter { id: "Nope".into() }),
        not_found("parameter", "Nope")
    );
}

#[test]
fn set_parameter_range() {
    let model = fixtures::single_quad();
    let cmd: Command = serde_json::from_str(
        r#"{"op":"set_parameter_range","id":"X","min":-2,"default":0,"max":2}"#,
    )
    .unwrap();
    let after = applied(&model, cmd);
    assert_eq!(after.parameters[0].max, 2.0);
    assert_eq!(
        rejected(
            &model,
            Command::SetParameterRange {
                id: "X".into(),
                min: 1.0,
                default: 0.0,
                max: 2.0,
            },
        ),
        EditError::InvalidRange {
            min: 1.0,
            default: 0.0,
            max: 2.0,
        }
    );
    assert_eq!(
        rejected(
            &model,
            Command::SetParameterRange {
                id: "Nope".into(),
                min: 0.0,
                default: 0.0,
                max: 0.0,
            },
        ),
        not_found("parameter", "Nope")
    );
}

#[test]
fn rename_parameter() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::RenameParameter {
            id: "X".into(),
            name: "Horizontal".into(),
        },
    );
    assert_eq!(after.parameters[0].name, "Horizontal");
    assert_eq!(after.parameters[0].id.as_str(), "X");
    assert_eq!(
        rejected(
            &model,
            Command::RenameParameter {
                id: "Nope".into(),
                name: "N".into(),
            },
        ),
        not_found("parameter", "Nope")
    );
}

#[test]
fn set_parameter_group() {
    let mut model = fixtures::single_quad();
    model.parameter_groups.push(ParameterGroup {
        id: "Face".into(),
        name: "Face".into(),
    });
    let after = applied(
        &model,
        Command::SetParameterGroup {
            id: "X".into(),
            group: Some("Face".into()),
        },
    );
    assert_eq!(
        after.parameters[0].group.as_ref().map(|g| g.as_str()),
        Some("Face")
    );
    let cleared = applied(
        &after,
        Command::SetParameterGroup {
            id: "X".into(),
            group: None,
        },
    );
    assert_eq!(cleared.parameters[0].group, None);
    rejected_invalid(
        &model,
        Command::SetParameterGroup {
            id: "X".into(),
            group: Some("Body".into()),
        },
        |e| matches!(e, ValidationError::UnknownReference { field: "group", .. }),
    );
}

fn group(id: &str, name: &str) -> ParameterGroup {
    ParameterGroup {
        id: id.into(),
        name: name.into(),
    }
}

fn group_ids(model: &Model) -> Vec<&str> {
    model
        .parameter_groups
        .iter()
        .map(|g| g.id.as_str())
        .collect()
}

#[test]
fn add_parameter_group() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::AddParameterGroup {
            group: group("Face", "顔"),
        },
    );
    assert_eq!(after.parameter_groups, vec![group("Face", "顔")]);
    assert_eq!(
        rejected(
            &after,
            Command::AddParameterGroup {
                group: group("Face", "Other"),
            },
        ),
        EditError::DuplicateId {
            kind: "parameter group",
            id: "Face".into(),
        }
    );
}

#[test]
fn remove_parameter_group_unassigns_its_parameters() {
    let mut model = fixtures::single_quad();
    model.parameter_groups = vec![group("Face", "Face"), group("Body", "Body")];
    model.parameters[0].group = Some("Face".into());
    let after = applied(&model, Command::RemoveParameterGroup { id: "Face".into() });
    assert_eq!(group_ids(&after), ["Body"]);
    // パラメータは残り、割り当てだけが外れる。
    assert_eq!(after.parameters.len(), 1);
    assert_eq!(after.parameters[0].group, None);
    assert_eq!(
        rejected(&model, Command::RemoveParameterGroup { id: "Nope".into() }),
        not_found("parameter group", "Nope")
    );
}

#[test]
fn rename_parameter_group() {
    let mut model = fixtures::single_quad();
    model.parameter_groups = vec![group("Face", "Face")];
    let after = applied(
        &model,
        Command::RenameParameterGroup {
            id: "Face".into(),
            name: "顔".into(),
        },
    );
    assert_eq!(after.parameter_groups, vec![group("Face", "顔")]);
    assert_eq!(
        rejected(
            &model,
            Command::RenameParameterGroup {
                id: "Nope".into(),
                name: "N".into(),
            },
        ),
        not_found("parameter group", "Nope")
    );
}

#[test]
fn reorder_parameter_group() {
    let mut model = fixtures::single_quad();
    model.parameter_groups = vec![group("A", "A"), group("B", "B"), group("C", "C")];
    let reorder = |id: &str, before: Option<&str>| Command::ReorderParameterGroup {
        id: id.into(),
        before: before.map(Into::into),
    };
    assert_eq!(
        group_ids(&applied(&model, reorder("C", Some("A")))),
        ["C", "A", "B"]
    );
    assert_eq!(
        group_ids(&applied(&model, reorder("A", None))),
        ["B", "C", "A"]
    );
    assert_eq!(
        rejected(&model, reorder("A", Some("A"))),
        EditError::SelfReference {
            kind: "parameter group",
            id: "A".into(),
            field: "before",
        }
    );
    assert_eq!(
        rejected(&model, reorder("Nope", None)),
        not_found("parameter group", "Nope")
    );
    assert_eq!(
        rejected(&model, reorder("A", Some("Nope"))),
        not_found("parameter group", "Nope")
    );
}

#[test]
fn parameter_group_commands_undo() {
    let model = fixtures::single_quad();
    let mut doc = Document::new(model.clone());
    doc.apply(Command::AddParameterGroup {
        group: group("Face", "Face"),
    })
    .unwrap();
    doc.apply(Command::SetParameterGroup {
        id: "X".into(),
        group: Some("Face".into()),
    })
    .unwrap();
    let assigned = doc.model().clone();
    doc.apply(Command::RemoveParameterGroup { id: "Face".into() })
        .unwrap();
    assert!(doc.model().parameter_groups.is_empty());
    // 消す前に戻すと、グループと割り当てがそろって戻る。
    assert!(doc.undo());
    assert_eq!(doc.model(), &assigned);
    assert!(doc.undo());
    assert!(doc.undo());
    assert_eq!(doc.model(), &model);
}

#[test]
fn set_parameter_mirror() {
    let model = fixtures::pose_and_effects();
    let mirror_of = |model: &Model, id: &str| {
        model
            .parameters
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .mirror
            .clone()
    };
    // 対は双方向に張る。
    let after = applied(
        &model,
        Command::SetParameterMirror {
            id: "EyeLOpen".into(),
            mirror: Some("EyeROpen".into()),
        },
    );
    assert_eq!(mirror_of(&after, "EyeLOpen"), Some("EyeROpen".into()));
    assert_eq!(mirror_of(&after, "EyeROpen"), Some("EyeLOpen".into()));
    // 片方を自分自身と対にすると、前の相手の対は外れる。
    let selfed = applied(
        &after,
        Command::SetParameterMirror {
            id: "EyeROpen".into(),
            mirror: Some("EyeROpen".into()),
        },
    );
    assert_eq!(mirror_of(&selfed, "EyeROpen"), Some("EyeROpen".into()));
    assert_eq!(mirror_of(&selfed, "EyeLOpen"), None);
    // `None` で外すと、相手の対も外れる。
    let cleared = applied(
        &after,
        Command::SetParameterMirror {
            id: "EyeLOpen".into(),
            mirror: None,
        },
    );
    assert_eq!(mirror_of(&cleared, "EyeLOpen"), None);
    assert_eq!(mirror_of(&cleared, "EyeROpen"), None);
    // 自分自身との対 (符号の反転) も許す。
    let after = applied(
        &model,
        Command::SetParameterMirror {
            id: "AngleX".into(),
            mirror: Some("AngleX".into()),
        },
    );
    assert_eq!(mirror_of(&after, "AngleX"), Some("AngleX".into()));
    assert_eq!(
        rejected(
            &model,
            Command::SetParameterMirror {
                id: "EyeLOpen".into(),
                mirror: Some("Nope".into()),
            },
        ),
        not_found("parameter", "Nope")
    );
}
