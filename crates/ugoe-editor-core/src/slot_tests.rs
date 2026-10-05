//! スロットのコマンド (`AddSlot`・`SetSlot`・`RemoveSlot`) と、削除の連鎖のテスト。
//! 規則は `docs/editor.md#スロットの編集`。

use ugoe_model::module::{DrawOrderRange, Slot, SlotId};
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::pose::{PoseEntry, PoseGroup};
use ugoe_model::validate::{ValidationError, validate};
use ugoe_model::{Model, fixtures};

use crate::physics_edit::default_settings;
use crate::slot_edit::ranges_overlap;
use crate::{Command, Document, EditError};

// ---- 補助 (tests.rs と同じ規則) ----

/// 適用に成功し、結果が検証を通り、Undo / Redo で前後を行き来できることを確かめて、適用後のモデルを返す。
#[track_caller]
fn applied(model: &Model, command: Command) -> Model {
    applied_with(model, None, command)
}

#[track_caller]
fn applied_with(model: &Model, physics: Option<PhysicsSettings>, command: Command) -> Model {
    assert!(validate(model).is_empty(), "fixture must be valid");
    let mut doc = Document::new(model.clone()).with_physics(physics);
    doc.apply(command).unwrap();
    let after = doc.model().clone();
    assert_eq!(validate(&after), vec![]);
    assert!(doc.undo());
    assert_eq!(doc.model(), model);
    assert!(doc.redo());
    assert_eq!(doc.model(), &after);
    after
}

/// 適用に失敗し、モデルと履歴が変わらないことを確かめて、エラーを返す。
#[track_caller]
fn rejected(model: &Model, command: Command) -> EditError {
    rejected_with(model, None, command)
}

#[track_caller]
fn rejected_with(model: &Model, physics: Option<PhysicsSettings>, command: Command) -> EditError {
    let mut doc = Document::new(model.clone()).with_physics(physics);
    let err = doc.apply(command).unwrap_err();
    assert_eq!(doc.model(), model);
    assert!(!doc.undo());
    err
}

/// 取り付け先も参照もない、描画順 `min..max` のスロット。
fn slot(id: &str, min: f32, max: f32) -> Slot {
    Slot {
        id: id.into(),
        anchor: None,
        draw_order: DrawOrderRange { min, max },
        replaces: Vec::new(),
        parameters: Vec::new(),
        mask_sources: Vec::new(),
        part: None,
        colliders: Vec::new(),
        pose_parts: Vec::new(),
    }
}

fn find<'a>(model: &'a Model, id: &str) -> &'a Slot {
    model
        .slots
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("slot {id} not found"))
}

/// スロット `Hair` を公開するアバタープロファイルを付けた `dress_up_base`。
fn with_profile() -> Model {
    let mut model = fixtures::dress_up_base();
    let mut profile = fixtures::avatar_profile().avatar.unwrap();
    profile.slots = vec!["Hair".into(), "Outfit".into()];
    model.avatar = Some(profile);
    model
}

/// 衝突体 `Collar` だけを持つ物理の設定。
fn physics_with_collar() -> PhysicsSettings {
    let mut settings = default_settings();
    settings.colliders = fixtures::long_hair_module().physics.colliders;
    settings
}

// ---- 追加 ----

#[test]
fn add_slot_appends_with_references() {
    let base = fixtures::dress_up_base();
    let mut hat = slot("Hat", 700.0, 750.0);
    hat.anchor = Some("Head".into());
    hat.part = Some("Root".into());
    hat.mask_sources = vec!["Face".into()];
    hat.parameters = vec!["Secret".into()];
    let after = applied(&base, Command::AddSlot { slot: hat.clone() });
    assert_eq!(after.slots.len(), 3);
    assert_eq!(after.slots[2], hat);
}

#[test]
fn add_slot_rejects_bad_ids_and_references() {
    let base = fixtures::dress_up_base();
    assert_eq!(
        rejected(
            &base,
            Command::AddSlot {
                slot: slot("Hair", 900.0, 950.0)
            }
        ),
        EditError::DuplicateId {
            kind: "slot",
            id: "Hair".into()
        }
    );
    for id in ["", "Head/Hat", "帽子"] {
        let err = rejected(
            &base,
            Command::AddSlot {
                slot: slot(id, 900.0, 950.0),
            },
        );
        assert!(matches!(err, EditError::InvalidSlot { .. }), "{id}: {err}");
    }
    // 参照先と値は適用後の検証が調べる。
    let mut hat = slot("Hat", 900.0, 950.0);
    hat.anchor = Some("NoDeformer".into());
    hat.replaces = vec!["NoPart".into()];
    let EditError::Invalid(errors) = rejected(&base, Command::AddSlot { slot: hat }) else {
        panic!("expected a validation error");
    };
    assert_eq!(errors.len(), 2);
    assert!(
        errors
            .iter()
            .all(|e| matches!(e, ValidationError::UnknownReference { kind: "slot", .. }))
    );
    let err = rejected(
        &base,
        Command::AddSlot {
            slot: slot("Hat", 950.0, 900.0),
        },
    );
    assert!(matches!(err, EditError::Invalid(_)), "{err}");
}

#[test]
fn add_slot_checks_colliders_against_physics() {
    let base = fixtures::dress_up_base();
    let mut hat = slot("Hat", 900.0, 950.0);
    hat.colliders = vec!["Collar".into()];
    let not_found = EditError::NotFound {
        kind: "physics collider",
        id: "Collar".into(),
    };
    // 物理の設定がなければ、衝突体は公開できない。
    assert_eq!(
        rejected(&base, Command::AddSlot { slot: hat.clone() }),
        not_found
    );
    let after = applied_with(
        &base,
        Some(physics_with_collar()),
        Command::AddSlot { slot: hat.clone() },
    );
    assert_eq!(find(&after, "Hat").colliders, ["Collar"]);
    hat.colliders = vec!["Nope".into()];
    assert_eq!(
        rejected_with(
            &base,
            Some(physics_with_collar()),
            Command::AddSlot { slot: hat }
        ),
        EditError::NotFound {
            kind: "physics collider",
            id: "Nope".into(),
        }
    );
}

#[test]
fn draw_order_ranges_must_not_overlap() {
    let base = fixtures::dress_up_base();
    // Hair は 550..650、Outfit は 300..400。
    for (min, max) in [
        (600.0, 700.0),
        (500.0, 560.0),
        (560.0, 600.0),
        (600.0, 600.0),
    ] {
        let err = rejected(
            &base,
            Command::AddSlot {
                slot: slot("Hat", min, max),
            },
        );
        let EditError::InvalidSlot { id, reason } = err else {
            panic!("{min}..{max}: expected InvalidSlot");
        };
        assert_eq!(id, "Hat");
        assert!(reason.contains("slot `Hair`"), "{reason}");
    }
    // 端が接するだけならよい。
    let after = applied(
        &base,
        Command::AddSlot {
            slot: slot("Hat", 650.0, 700.0),
        },
    );
    assert_eq!(find(&after, "Hat").draw_order.min, 650.0);
    // 幅 0 の範囲も、ほかの範囲の端に接するだけならよい。
    applied(
        &base,
        Command::AddSlot {
            slot: slot("Belt", 400.0, 450.0),
        },
    );
}

#[test]
fn overlap_is_symmetric_and_handles_empty_ranges() {
    let r = |min, max| DrawOrderRange { min, max };
    let cases = [
        (r(0.0, 10.0), r(5.0, 15.0), true),
        (r(0.0, 10.0), r(10.0, 20.0), false),
        (r(0.0, 10.0), r(0.0, 10.0), true),
        (r(5.0, 5.0), r(0.0, 10.0), true),
        (r(10.0, 10.0), r(0.0, 10.0), true),
        (r(5.0, 5.0), r(5.0, 5.0), true),
        (r(5.0, 5.0), r(6.0, 6.0), false),
        (r(0.0, 1.0), r(2.0, 3.0), false),
    ];
    for (a, b, expected) in cases {
        assert_eq!(ranges_overlap(a, b), expected, "{a:?} {b:?}");
        assert_eq!(ranges_overlap(b, a), expected, "{b:?} {a:?}");
    }
}

// ---- 置き換え ----

#[test]
fn set_slot_replaces_fields() {
    let base = fixtures::dress_up_base();
    let mut hair = find(&base, "Hair").clone();
    hair.draw_order = DrawOrderRange {
        min: 520.0,
        max: 680.0,
    };
    hair.replaces.clear();
    hair.part = Some("Root".into());
    let after = applied(
        &base,
        Command::SetSlot {
            id: "Hair".into(),
            slot: hair.clone(),
        },
    );
    assert_eq!(find(&after, "Hair"), &hair);
    // 並びは変わらない。
    assert_eq!(after.slots[0].id.as_str(), "Hair");
}

#[test]
fn set_slot_rename_follows_avatar_profile() {
    let base = with_profile();
    let mut hair = find(&base, "Hair").clone();
    hair.id = "Hairstyle".into();
    let after = applied(
        &base,
        Command::SetSlot {
            id: "Hair".into(),
            slot: hair,
        },
    );
    assert_eq!(after.slots[0].id.as_str(), "Hairstyle");
    assert_eq!(
        after.avatar.unwrap().slots,
        [SlotId::from("Hairstyle"), "Outfit".into()]
    );

    let mut hair = find(&base, "Hair").clone();
    hair.id = "Outfit".into();
    assert_eq!(
        rejected(
            &base,
            Command::SetSlot {
                id: "Hair".into(),
                slot: hair.clone(),
            }
        ),
        EditError::DuplicateId {
            kind: "slot",
            id: "Outfit".into()
        }
    );
    assert_eq!(
        rejected(
            &base,
            Command::SetSlot {
                id: "Nope".into(),
                slot: hair,
            }
        ),
        EditError::NotFound {
            kind: "slot",
            id: "Nope".into()
        }
    );
}

#[test]
fn set_slot_rejects_new_overlaps_but_keeps_loaded_ones_editable() {
    let base = fixtures::dress_up_base();
    let mut outfit = find(&base, "Outfit").clone();
    outfit.draw_order.max = 600.0;
    let err = rejected(
        &base,
        Command::SetSlot {
            id: "Outfit".into(),
            slot: outfit.clone(),
        },
    );
    assert!(matches!(err, EditError::InvalidSlot { .. }), "{err}");

    // 読み込んだ時点で重なっていても、ほかの項目は変えられる。
    let mut loaded = base.clone();
    loaded.slots[1].draw_order.max = 600.0;
    outfit.parameters.clear();
    let after = applied(
        &loaded,
        Command::SetSlot {
            id: "Outfit".into(),
            slot: outfit,
        },
    );
    assert!(find(&after, "Outfit").parameters.is_empty());
}

// ---- 削除 ----

#[test]
fn remove_slot_drops_it_from_the_avatar_profile() {
    let base = with_profile();
    let after = applied(&base, Command::RemoveSlot { id: "Hair".into() });
    assert_eq!(after.slots.len(), 1);
    assert_eq!(after.avatar.unwrap().slots, ["Outfit"]);
    assert_eq!(
        rejected(&base, Command::RemoveSlot { id: "Hair2".into() }),
        EditError::NotFound {
            kind: "slot",
            id: "Hair2".into()
        }
    );
}

// ---- 削除の連鎖と拒否 ----

#[test]
fn removing_referenced_elements_cascades_into_slots() {
    let base = fixtures::dress_up_base();
    // メッシュ: マスクに使ってよいメッシュから外す。
    let after = applied(&base, Command::RemoveMesh { id: "Face".into() });
    assert!(find(&after, "Hair").mask_sources.is_empty());
    // パラメータ: 公開するパラメータから外す。
    let after = applied(
        &base,
        Command::RemoveParameter {
            id: "BodySway".into(),
        },
    );
    assert!(find(&after, "Outfit").parameters.is_empty());
}

#[test]
fn removing_parts_cascades_into_replaces_and_pose_parts() {
    let mut base = fixtures::dress_up_base();
    let arm = |id: &str| {
        let mut part = base.parts[1].clone();
        part.id = id.into();
        part.name = id.into();
        part
    };
    let (a, b, hat) = (arm("ArmA"), arm("ArmB"), arm("Hat"));
    base.parts.extend([a, b, hat]);
    base.pose.groups.push(PoseGroup {
        entries: vec![
            PoseEntry {
                part: "ArmA".into(),
                links: Vec::new(),
            },
            PoseEntry {
                part: "ArmB".into(),
                links: Vec::new(),
            },
        ],
    });
    base.slots[1].pose_parts = vec!["ArmA".into(), "ArmB".into()];
    base.slots[0].replaces.push("Hat".into());

    let after = applied(&base, Command::RemovePart { id: "ArmA".into() });
    assert_eq!(find(&after, "Outfit").pose_parts, ["ArmB"]);
    let after = applied(&base, Command::RemovePart { id: "Hat".into() });
    assert_eq!(find(&after, "Hair").replaces, ["Hair"]);
}

#[test]
fn removing_an_anchor_or_parent_part_is_rejected() {
    let mut base = fixtures::dress_up_base();
    let mut empty = base.parts[1].clone();
    empty.id = "Empty".into();
    base.parts.push(empty);
    base.slots[1].part = Some("Empty".into());
    assert_eq!(
        rejected(&base, Command::RemovePart { id: "Empty".into() }),
        EditError::HasChildren {
            kind: "part",
            id: "Empty".into(),
            children: vec!["slot `Outfit`".into()],
        }
    );
    let EditError::HasChildren { children, .. } =
        rejected(&base, Command::RemoveDeformer { id: "Torso".into() })
    else {
        panic!("expected HasChildren");
    };
    assert!(
        children.contains(&"slot `Outfit`".to_owned()),
        "{children:?}"
    );
}

#[test]
fn removing_a_collider_cascades_into_slots() {
    let mut base = fixtures::dress_up_base();
    base.slots[0].colliders = vec!["Collar".into()];
    let after = applied_with(
        &base,
        Some(physics_with_collar()),
        Command::RemovePhysicsCollider {
            id: "Collar".into(),
        },
    );
    assert!(find(&after, "Hair").colliders.is_empty());
}
