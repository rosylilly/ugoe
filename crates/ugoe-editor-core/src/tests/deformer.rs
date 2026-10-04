//! デフォーマのコマンドのテスト。

use super::*;

#[test]
fn add_deformer() {
    let model = fixtures::rotation_chain();
    let after = applied(
        &model,
        Command::AddDeformer {
            deformer: new_warp("Skirt", Some("Arm")),
        },
    );
    assert_eq!(after.deformers.len(), 4);
    assert_eq!(
        rejected(
            &model,
            Command::AddDeformer {
                deformer: new_warp("Arm", None),
            },
        ),
        EditError::DuplicateId {
            kind: "deformer",
            id: "Arm".into(),
        }
    );
    rejected_invalid(
        &model,
        Command::AddDeformer {
            deformer: new_warp("Skirt", Some("Nope")),
        },
        |e| matches!(e, ValidationError::UnknownReference { .. }),
    );
}

#[test]
fn remove_deformer() {
    let model = fixtures::rotation_chain();
    assert_eq!(
        rejected(&model, Command::RemoveDeformer { id: "Arm".into() }),
        EditError::HasChildren {
            kind: "deformer",
            id: "Arm".into(),
            children: vec!["deformer `Hand`".into(), "deformer `Cloth`".into()],
        }
    );
    assert_eq!(
        rejected(&model, Command::RemoveDeformer { id: "Hand".into() }),
        EditError::HasChildren {
            kind: "deformer",
            id: "Hand".into(),
            children: vec!["mesh `Palm`".into()],
        }
    );
    assert_eq!(
        rejected(&model, Command::RemoveDeformer { id: "Nope".into() }),
        not_found("deformer", "Nope")
    );
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
    assert_eq!(after.deformers.len(), 2);
}

#[test]
fn rename_deformer() {
    let model = fixtures::rotation_chain();
    let after = applied(
        &model,
        Command::RenameDeformer {
            id: "Arm".into(),
            name: "Upper arm".into(),
        },
    );
    assert_eq!(after.deformers[0].name, "Upper arm");
    assert_eq!(
        rejected(
            &model,
            Command::RenameDeformer {
                id: "Nope".into(),
                name: "N".into(),
            },
        ),
        not_found("deformer", "Nope")
    );
}

#[test]
fn set_deformer_parent_and_part() {
    let model = fixtures::rotation_chain();
    let after = applied(
        &model,
        Command::SetDeformerParent {
            id: "Cloth".into(),
            parent: None,
        },
    );
    assert_eq!(after.deformers[2].parent, None);
    rejected_invalid(
        &model,
        Command::SetDeformerParent {
            id: "Arm".into(),
            parent: Some("Hand".into()),
        },
        |e| {
            matches!(
                e,
                ValidationError::ParentCycle {
                    kind: "deformer",
                    ..
                }
            )
        },
    );

    let mut with_part = model.clone();
    with_part.parts.push(new_part("Body", Some("Root")));
    let after = applied(
        &with_part,
        Command::SetDeformerPart {
            id: "Arm".into(),
            part: "Body".into(),
        },
    );
    assert_eq!(after.deformers[0].part.as_str(), "Body");
    rejected_invalid(
        &model,
        Command::SetDeformerPart {
            id: "Arm".into(),
            part: "Nope".into(),
        },
        |e| matches!(e, ValidationError::UnknownReference { field: "part", .. }),
    );
}

#[test]
fn set_deformer_keyforms() {
    let model = fixtures::rotation_chain();
    let DeformerKind::Warp(cloth) = rotation_chain_deformer("Cloth").kind else {
        panic!("Cloth is a warp deformer");
    };
    let rest = cloth.keyforms.keyforms[0].clone();
    let mut moved = rest.clone();
    moved.points[0].x += 1.0;
    let warp = grid1("Flip", &[0.0, 1.0], vec![rest.clone(), moved]);
    let after = applied(
        &model,
        Command::SetWarpKeyforms {
            id: "Cloth".into(),
            keyforms: warp.clone(),
        },
    );
    let DeformerKind::Warp(cloth) = &after.deformers[2].kind else {
        panic!("Cloth is a warp deformer");
    };
    assert_eq!(cloth.keyforms, warp);
    assert_eq!(
        rejected(
            &model,
            Command::SetWarpKeyforms {
                id: "Arm".into(),
                keyforms: warp,
            },
        ),
        EditError::DeformerKindMismatch {
            id: "Arm".into(),
            expected: "warp",
        }
    );
    let mut short = rest;
    short.points.pop();
    rejected_invalid(
        &model,
        Command::SetWarpKeyforms {
            id: "Cloth".into(),
            keyforms: KeyformGrid::constant(short),
        },
        |e| matches!(e, ValidationError::WarpPointCount { .. }),
    );

    let DeformerKind::Rotation(hand) = rotation_chain_deformer("Hand").kind else {
        panic!("Hand is a rotation deformer");
    };
    let rotation = KeyformGrid::constant(hand.keyforms.keyforms[1]);
    let after = applied(
        &model,
        Command::SetRotationKeyforms {
            id: "Hand".into(),
            keyforms: rotation.clone(),
        },
    );
    let DeformerKind::Rotation(hand) = &after.deformers[1].kind else {
        panic!("Hand is a rotation deformer");
    };
    assert_eq!(hand.keyforms, rotation);
    assert_eq!(
        rejected(
            &model,
            Command::SetRotationKeyforms {
                id: "Cloth".into(),
                keyforms: rotation,
            },
        ),
        EditError::DeformerKindMismatch {
            id: "Cloth".into(),
            expected: "rotation",
        }
    );
}

#[test]
fn unknown_deformer_kinds_are_not_edited() {
    // 新しい版のファイルから読んだ、未知の種類のデフォーマ (`docs/spec/file-formats.md#未知の値の読み込み`)。
    let mut model = fixtures::rotation_chain();
    model.deformers[1].kind = DeformerKind::Unknown;
    let DeformerKind::Warp(cloth) = rotation_chain_deformer("Cloth").kind else {
        panic!("Cloth is a warp deformer");
    };
    let DeformerKind::Rotation(hand) = rotation_chain_deformer("Hand").kind else {
        panic!("Hand is a rotation deformer");
    };
    assert_eq!(
        rejected(
            &model,
            Command::SetWarpKeyforms {
                id: "Hand".into(),
                keyforms: cloth.keyforms,
            },
        ),
        EditError::DeformerKindMismatch {
            id: "Hand".into(),
            expected: "warp",
        }
    );
    assert_eq!(
        rejected(
            &model,
            Command::SetRotationKeyforms {
                id: "Hand".into(),
                keyforms: hand.keyforms,
            },
        ),
        EditError::DeformerKindMismatch {
            id: "Hand".into(),
            expected: "rotation",
        }
    );
    // パラメータの削除は、キーフォームを持たない未知のデフォーマには触れない。
    let after = applied(&model, Command::RemoveParameter { id: "Flip".into() });
    assert_eq!(after.deformers[1].kind, DeformerKind::Unknown);
}
