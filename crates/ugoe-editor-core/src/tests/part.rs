//! パーツのコマンドのテスト。

use super::*;

#[test]
fn add_part() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::AddPart {
            part: new_part("Head", Some("Root")),
        },
    );
    assert_eq!(after.parts[1].id.as_str(), "Head");
    assert_eq!(
        rejected(
            &model,
            Command::AddPart {
                part: new_part("Root", None),
            },
        ),
        EditError::DuplicateId {
            kind: "part",
            id: "Root".into(),
        }
    );
    rejected_invalid(
        &model,
        Command::AddPart {
            part: new_part("Head", Some("Nope")),
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
fn remove_part_rejects_parts_with_children() {
    let model = fixtures::pose_and_effects();
    assert_eq!(
        rejected(&model, Command::RemovePart { id: "Root".into() }),
        EditError::HasChildren {
            kind: "part",
            id: "Root".into(),
            children: vec![
                "part `ArmA`".into(),
                "part `SleeveA`".into(),
                "part `ArmB`".into(),
                "mesh `Quad`".into(),
            ],
        }
    );
    let model = fixtures::rotation_chain();
    let err = rejected(&model, Command::RemovePart { id: "Root".into() });
    assert!(
        matches!(&err, EditError::HasChildren { children, .. } if children[0] == "deformer `Arm`"),
        "{err:?}"
    );
    assert_eq!(
        rejected(&model, Command::RemovePart { id: "Nope".into() }),
        not_found("part", "Nope")
    );
}

#[test]
fn remove_part_drops_pose_references() {
    let model = fixtures::pose_and_effects();
    let after = applied(&model, Command::RemovePart { id: "ArmA".into() });
    assert_eq!(after.parts.len(), 3);
    let entries = &after.pose.groups[0].entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].part.as_str(), "ArmB");

    let after = applied(
        &model,
        Command::RemovePart {
            id: "SleeveA".into(),
        },
    );
    assert!(after.pose.groups[0].entries[0].links.is_empty());

    // グループが空になったらグループごと消える。
    let after = applied(
        &model,
        Command::Batch {
            label: "remove arms".into(),
            commands: vec![
                Command::RemovePart { id: "ArmA".into() },
                Command::RemovePart { id: "ArmB".into() },
            ],
        },
    );
    assert!(after.pose.groups.is_empty());
}

#[test]
fn rename_part() {
    let model = fixtures::single_quad();
    let after = applied(
        &model,
        Command::RenamePart {
            id: "Root".into(),
            name: "Body".into(),
        },
    );
    assert_eq!(after.parts[0].name, "Body");
    assert_eq!(
        rejected(
            &model,
            Command::RenamePart {
                id: "Nope".into(),
                name: "N".into(),
            },
        ),
        not_found("part", "Nope")
    );
}

#[test]
fn set_part_parent() {
    let model = fixtures::pose_and_effects();
    let after = applied(
        &model,
        Command::SetPartParent {
            id: "SleeveA".into(),
            parent: Some("ArmA".into()),
        },
    );
    assert_eq!(after.parts[2].parent, Some("ArmA".into()));
    rejected_invalid(
        &model,
        Command::SetPartParent {
            id: "Root".into(),
            parent: Some("ArmA".into()),
        },
        |e| matches!(e, ValidationError::ParentCycle { kind: "part", .. }),
    );
    rejected_invalid(
        &model,
        Command::SetPartParent {
            id: "ArmA".into(),
            parent: Some("Nope".into()),
        },
        |e| matches!(e, ValidationError::UnknownReference { .. }),
    );
}

#[test]
fn set_part_keyforms() {
    let model = fixtures::single_quad();
    let kf = model.parts[0].keyforms.keyforms[0];
    let hidden = ugoe_model::part::PartKeyform { opacity: 0.0 };
    let after = applied(
        &model,
        Command::SetPartKeyforms {
            id: "Root".into(),
            keyforms: grid1("X", &[0.0, 1.0], vec![kf, hidden]),
        },
    );
    assert_eq!(after.parts[0].keyforms.keyforms.len(), 2);
    rejected_invalid(
        &model,
        Command::SetPartKeyforms {
            id: "Root".into(),
            keyforms: grid1("X", &[0.0, 1.0], vec![kf]),
        },
        |e| matches!(e, ValidationError::KeyformCount { .. }),
    );
}
