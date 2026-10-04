//! メッシュのコマンドのテスト。

use super::*;

#[test]
fn add_mesh() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::AddMesh {
            mesh: new_mesh("Quad2"),
        },
    );
    assert_eq!(mesh_ids(&after), ["Quad", "Quad2"]);
    assert_eq!(
        rejected(
            &model,
            Command::AddMesh {
                mesh: new_mesh("Quad"),
            },
        ),
        EditError::DuplicateId {
            kind: "mesh",
            id: "Quad".into(),
        }
    );
    let mut bad = new_mesh("Quad2");
    bad.texture = 5;
    rejected_invalid(&model, Command::AddMesh { mesh: bad }, |e| {
        matches!(e, ValidationError::TextureOutOfRange { .. })
    });
}

#[test]
fn remove_mesh_drops_mask_references() {
    let model = fixtures::nested_masks();
    let after = applied(&model, Command::RemoveMesh { id: "Iris".into() });
    assert_eq!(mesh_ids(&after), ["Pupil", "Sclera"]);
    assert!(after.meshes[0].masks.is_empty());
    assert_eq!(
        rejected(&model, Command::RemoveMesh { id: "Nope".into() }),
        not_found("mesh", "Nope")
    );
}

#[test]
fn remove_mesh_and_deformer_drop_mirror_references() {
    let mut model = fixtures::glued_quads();
    model.meshes[0].mirror = Some("Right".into());
    model.meshes[1].mirror = Some("Left".into());
    model.glues.clear();
    let after = applied(&model, Command::RemoveMesh { id: "Left".into() });
    assert_eq!(after.meshes[0].mirror, None);

    let mut model = fixtures::rotation_chain();
    model.deformers[1].mirror = Some("Cloth".into());
    model.deformers[2].mirror = Some("Hand".into());
    let after = applied(
        &model,
        Command::Batch {
            label: "remove hand".into(),
            commands: vec![
                Command::RemoveMesh { id: "Palm".into() },
                Command::RemoveDeformer { id: "Hand".into() },
            ],
        },
    );
    assert_eq!(after.deformers[1].id.as_str(), "Cloth");
    assert_eq!(after.deformers[1].mirror, None);
}

#[test]
fn remove_mesh_drops_glues_and_hit_areas() {
    let mut model = fixtures::glued_quads();
    model.hit_areas.push(HitArea {
        id: "Touch".into(),
        mesh: "Left".into(),
    });
    let after = applied(&model, Command::RemoveMesh { id: "Left".into() });
    assert!(after.glues.is_empty());
    assert!(after.hit_areas.is_empty());
}

#[test]
fn rename_mesh() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::RenameMesh {
            id: "Quad".into(),
            name: "Square".into(),
        },
    );
    assert_eq!(after.meshes[0].name, "Square");
    assert_eq!(
        rejected(
            &model,
            Command::RenameMesh {
                id: "Nope".into(),
                name: "N".into(),
            },
        ),
        not_found("mesh", "Nope")
    );
}

#[test]
fn set_mesh_properties_changes_only_given_fields() {
    let model = fixtures::nested_masks();
    let cmd: Command =
        serde_json::from_str(r#"{"op":"set_mesh_properties","id":"Iris","blend":"additive"}"#)
            .unwrap();
    let after = applied(&model, cmd);
    let mut expected = model.meshes[1].clone();
    expected.blend = BlendMode::Additive;
    assert_eq!(after.meshes[1], expected);

    let mut with_part = model.clone();
    with_part.parts.push(new_part("Eye", Some("Root")));
    let after = applied(
        &with_part,
        Command::SetMeshProperties {
            id: "Pupil".into(),
            part: Some("Eye".into()),
            texture: None,
            texture_name: Some("white".into()),
            blend: Some(BlendMode::Multiply),
            culling: Some(true),
            masks: Some(vec![MaskRef::inverted("Sclera")]),
            mask_mode: Some(ugoe_model::MaskMode::Intersection),
            mask_only: Some(true),
            lit: None,
        },
    );
    let pupil = &after.meshes[0];
    assert_eq!(pupil.part.as_str(), "Eye");
    assert_eq!(pupil.blend, BlendMode::Multiply);
    assert!(pupil.culling && pupil.mask_only);
    assert_eq!(pupil.masks, vec![MaskRef::inverted("Sclera")]);
    assert_eq!(pupil.mask_mode, ugoe_model::MaskMode::Intersection);
}

#[test]
fn set_mesh_properties_rejects_invalid_results() {
    let model = fixtures::nested_masks();
    let props = |masks, texture| Command::SetMeshProperties {
        id: "Sclera".into(),
        part: None,
        texture,
        texture_name: None,
        blend: None,
        culling: None,
        masks,
        mask_mode: None,
        mask_only: None,
        lit: None,
    };
    rejected_invalid(
        &model,
        props(Some(vec![MaskRef::normal("Pupil")]), None),
        |e| matches!(e, ValidationError::MaskCycle { .. }),
    );
    rejected_invalid(&model, props(None, Some(1)), |e| {
        matches!(e, ValidationError::TextureOutOfRange { .. })
    });
    assert_eq!(
        rejected(
            &model,
            Command::SetMeshProperties {
                id: "Nope".into(),
                part: None,
                texture: None,
                texture_name: None,
                blend: None,
                culling: None,
                masks: None,
                mask_mode: None,
                mask_only: None,
                lit: None,
            },
        ),
        not_found("mesh", "Nope")
    );
}

/// `texture_name` だけを指定した `set_mesh_properties`。
fn set_texture(id: &str, name: &str) -> Command {
    serde_json::from_value(serde_json::json!({
        "op": "set_mesh_properties",
        "id": id,
        "texture_name": name,
    }))
    .unwrap()
}

#[test]
fn set_mesh_properties_gives_the_texture_by_name() {
    let mut model = fixtures::nested_masks();
    let mut second = model.textures[0].clone();
    second.name = "second".into();
    model.textures.push(second);

    let after = applied(&model, set_texture("Sclera", "second"));
    assert_eq!(after.meshes[2].id.as_str(), "Sclera");
    assert_eq!(after.meshes[2].texture, 1);

    assert_eq!(
        rejected(&model, set_texture("Sclera", "missing")),
        not_found("texture", "missing")
    );
    // 名前は大文字小文字まで一致させる (`TextureRef::name` と同じ)。
    assert_eq!(
        rejected(&model, set_texture("Sclera", "Second")),
        not_found("texture", "Second")
    );
}

#[test]
fn set_mesh_properties_still_reads_the_deprecated_texture_index() {
    let mut model = fixtures::nested_masks();
    let mut second = model.textures[0].clone();
    second.name = "second".into();
    model.textures.push(second);

    // 以前の形 (番号) で書かれたコマンドも読めて、その番号を設定する。
    let old: Command =
        serde_json::from_str(r#"{"op":"set_mesh_properties","id":"Sclera","texture":1}"#).unwrap();
    assert_eq!(applied(&model, old).meshes[2].texture, 1);

    let both: Command = serde_json::from_str(
        r#"{"op":"set_mesh_properties","id":"Sclera","texture":1,"texture_name":"second"}"#,
    )
    .unwrap();
    assert_eq!(
        rejected(&model, both),
        EditError::ConflictingTexture {
            id: "Sclera".into()
        }
    );
}

#[test]
fn set_mesh_parent() {
    let model = fixtures::warp_grid();
    let after = applied(
        &model,
        Command::SetMeshParent {
            id: "Inner".into(),
            parent: None,
        },
    );
    assert_eq!(after.meshes[0].parent, None);
    rejected_invalid(
        &model,
        Command::SetMeshParent {
            id: "Inner".into(),
            parent: Some("Nope".into()),
        },
        |e| {
            matches!(
                e,
                ValidationError::UnknownReference {
                    field: "parent",
                    ..
                }
            )
        },
    );
}

#[test]
fn set_mesh_keyforms() {
    let model = fixtures::single_quad();
    let rest = model.meshes[0].keyforms.keyforms[1].clone();
    let after = applied(
        &model,
        Command::SetMeshKeyforms {
            id: "Quad".into(),
            keyforms: KeyformGrid::constant(rest.clone()),
        },
    );
    assert_eq!(after.meshes[0].keyforms.keyforms, vec![rest.clone()]);
    let mut bad = rest;
    bad.positions.pop();
    rejected_invalid(
        &model,
        Command::SetMeshKeyforms {
            id: "Quad".into(),
            keyforms: KeyformGrid::constant(bad),
        },
        |e| matches!(e, ValidationError::VertexCount { .. }),
    );
}

#[test]
fn set_mesh_keyform_marks_cell_authored() {
    use ugoe_model::KeyformOrigin::{Additive, Authored, Derived};
    let mut model = fixtures::single_quad();
    // 基準のセル (X = 0) は推定の元なので `authored` のまま。
    model.meshes[0].keyforms.origins = vec![Derived, Authored, Derived];
    let keyform = model.meshes[0].keyforms.keyforms[2].clone();
    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "Quad".into(),
            cell: cell(&[("X", 1.0)]),
            keyform,
        },
    );
    assert_eq!(
        after.meshes[0].keyforms.origins,
        vec![Additive, Authored, Authored]
    );
}

#[test]
fn set_mesh_keyform_cell() {
    let model = fixtures::single_quad();
    let mut keyform = model.meshes[0].keyforms.keyforms[2].clone();
    keyform.opacity = 0.5;
    let after = applied(
        &model,
        Command::SetMeshKeyform {
            id: "Quad".into(),
            cell: cell(&[("X", 1.0)]),
            keyform: keyform.clone(),
        },
    );
    let mut expected = model.meshes[0].keyforms.clone();
    expected.keyforms[2] = keyform.clone();
    assert_eq!(after.meshes[0].keyforms, expected);

    assert_eq!(
        rejected(
            &model,
            Command::SetMeshKeyform {
                id: "Quad".into(),
                cell: cell(&[("X", 0.5)]),
                keyform: keyform.clone(),
            },
        ),
        EditError::InvalidCell {
            kind: "mesh",
            id: "Quad".into(),
            error: CellError::UnknownKey {
                parameter: "X".into(),
                value: 0.5,
            },
        }
    );
    let mut bad = keyform;
    bad.positions.pop();
    rejected_invalid(
        &model,
        Command::SetMeshKeyform {
            id: "Quad".into(),
            cell: cell(&[("X", 1.0)]),
            keyform: bad,
        },
        |e| matches!(e, ValidationError::VertexCount { .. }),
    );
}

#[test]
fn reorder_mesh() {
    let model = fixtures::nested_masks();
    assert_eq!(mesh_ids(&model), ["Pupil", "Iris", "Sclera"]);
    let after = applied(
        &model,
        Command::ReorderMesh {
            id: "Sclera".into(),
            before: Some("Pupil".into()),
        },
    );
    assert_eq!(mesh_ids(&after), ["Sclera", "Pupil", "Iris"]);
    let after = applied(
        &model,
        Command::ReorderMesh {
            id: "Pupil".into(),
            before: None,
        },
    );
    assert_eq!(mesh_ids(&after), ["Iris", "Sclera", "Pupil"]);
    let after = applied(
        &model,
        Command::ReorderMesh {
            id: "Pupil".into(),
            before: Some("Sclera".into()),
        },
    );
    assert_eq!(mesh_ids(&after), ["Iris", "Pupil", "Sclera"]);

    assert_eq!(
        rejected(
            &model,
            Command::ReorderMesh {
                id: "Iris".into(),
                before: Some("Iris".into()),
            },
        ),
        EditError::SelfReference {
            kind: "mesh",
            id: "Iris".into(),
            field: "before",
        }
    );
    assert_eq!(
        rejected(
            &model,
            Command::ReorderMesh {
                id: "Iris".into(),
                before: Some("Nope".into()),
            },
        ),
        not_found("mesh", "Nope")
    );
}
