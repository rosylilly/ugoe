//! アバタープロファイルと当たり判定のコマンド (`SetAvatarProfile`・`UpdateAvatarProfile`・`SetHitAreas`) と
//! 準拠の検査 (`avatar::check`) のテスト。規則は `docs/editor.md#アバタープロファイルの編集`。

use ugoe_model::avatar::{
    AvatarHitArea, AvatarMeta, AvatarPermission, AvatarProfile, CommercialUsage, CreditNotation,
    HIT_AREA_HEAD, LookAt, Modification, TrackingOverride, TrackingSource,
};
use ugoe_model::validate::validate;
use ugoe_model::{HitArea, Model, fixtures};

use crate::{Command, Document, EditError, MessageArg, avatar};

// ---- 補助 (tests.rs と同じ規則) ----

/// 適用に成功し、結果が検証を通り、Undo / Redo で前後を行き来できることを確かめて、適用後のモデルを返す。
#[track_caller]
fn applied(model: &Model, command: Command) -> Model {
    assert!(validate(model).is_empty(), "fixture must be valid");
    let mut doc = Document::new(model.clone());
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
    let mut doc = Document::new(model.clone());
    let err = doc.apply(command).unwrap_err();
    assert_eq!(doc.model(), model);
    assert!(!doc.undo());
    err
}

/// 何も変えない `UpdateAvatarProfile`。
fn update() -> Command {
    Command::UpdateAvatarProfile {
        level: None,
        meta: None,
        tracking: None,
        look_at: None,
        hit_areas: None,
        slots: None,
    }
}

fn with_update(f: impl FnOnce(&mut Command)) -> Command {
    let mut command = update();
    f(&mut command);
    command
}

fn head_hit_area(model: &mut Model) {
    model.hit_areas.push(HitArea {
        id: "HeadArea".into(),
        mesh: "Quad".into(),
    });
}

fn head_mapping() -> AvatarHitArea {
    AvatarHitArea {
        name: HIT_AREA_HEAD.into(),
        hit_area: "HeadArea".into(),
    }
}

fn tracking(scale: f32) -> TrackingOverride {
    TrackingOverride {
        source: TrackingSource::Standard,
        input: "EyeBallX".into(),
        parameter: None,
        scale,
        offset: 0.0,
    }
}

// ---- SetAvatarProfile ----

#[test]
fn set_profile_replaces_and_removes() {
    let fixture = fixtures::avatar_profile();
    let mut model = fixture.clone();
    model.avatar = None;
    let after = applied(
        &model,
        Command::SetAvatarProfile {
            profile: fixture.avatar.clone(),
        },
    );
    assert_eq!(after, fixture);
    let removed = applied(&after, Command::SetAvatarProfile { profile: None });
    assert_eq!(removed.avatar, None);
}

#[test]
fn set_profile_rejects_invalid_values() {
    let model = fixtures::avatar_profile();
    let base = model.avatar.clone().unwrap();
    let cases = [
        AvatarProfile {
            level: 0,
            ..base.clone()
        },
        AvatarProfile {
            level: 4,
            ..base.clone()
        },
        AvatarProfile {
            meta: AvatarMeta {
                modification: Modification::Unknown,
                ..base.meta.clone()
            },
            ..base.clone()
        },
        AvatarProfile {
            tracking: vec![tracking(f32::NAN)],
            ..base.clone()
        },
        AvatarProfile {
            look_at: Some(LookAt {
                range_x: 0.0,
                ..LookAt::default()
            }),
            ..base.clone()
        },
        AvatarProfile {
            hit_areas: vec![head_mapping(), head_mapping()],
            ..base.clone()
        },
    ];
    for profile in cases {
        let err = rejected(
            &model,
            Command::SetAvatarProfile {
                profile: Some(profile),
            },
        );
        assert!(
            matches!(err, EditError::InvalidAvatarProfile { .. }),
            "{err:?}"
        );
    }
}

/// 参照先 (パラメータ・当たり判定・スロット) がなくても拒否しない。準拠の検査がエラーとして示す。
#[test]
fn set_profile_accepts_dangling_references() {
    let model = fixtures::avatar_profile();
    let profile = AvatarProfile {
        tracking: vec![TrackingOverride {
            parameter: Some("Missing".into()),
            ..tracking(1.0)
        }],
        hit_areas: vec![head_mapping()],
        slots: vec!["Hat".into()],
        ..model.avatar.clone().unwrap()
    };
    let after = applied(
        &model,
        Command::SetAvatarProfile {
            profile: Some(profile),
        },
    );
    let check = avatar::check(&after);
    assert_eq!(check.errors.len(), 3, "{:?}", check.errors);
}

// ---- UpdateAvatarProfile ----

#[test]
fn update_creates_profile_with_most_restrictive_defaults() {
    let model = fixtures::single_quad();
    let after = applied(&model, update());
    let profile = after.avatar.unwrap();
    assert_eq!(profile, AvatarProfile::default());
    let meta = profile.meta;
    assert_eq!(meta.avatar_permission, AvatarPermission::OnlyAuthor);
    assert_eq!(meta.commercial_usage, CommercialUsage::PersonalNonProfit);
    assert_eq!(meta.credit_notation, CreditNotation::Required);
    assert_eq!(meta.modification, Modification::Prohibited);
    assert!(!meta.allow_redistribution);
    assert!(!meta.allow_violent_usage);
}

#[test]
fn update_changes_only_given_fields() {
    let mut model = fixtures::avatar_profile();
    head_hit_area(&mut model);
    let before = model.avatar.clone().unwrap();
    let meta = AvatarMeta {
        name: "Renamed".into(),
        commercial_usage: CommercialUsage::Corporation,
        ..before.meta.clone()
    };
    let after = applied(
        &model,
        with_update(|c| {
            if let Command::UpdateAvatarProfile {
                meta: m,
                hit_areas,
                look_at,
                ..
            } = c
            {
                *m = Some(meta.clone());
                *hit_areas = Some(vec![head_mapping()]);
                *look_at = Some(Some(LookAt::default()));
            }
        }),
    );
    let profile = after.avatar.unwrap();
    assert_eq!(profile.meta, meta);
    assert_eq!(profile.hit_areas, vec![head_mapping()]);
    assert_eq!(profile.look_at, Some(LookAt::default()));
    assert_eq!(profile.level, before.level);
    assert_eq!(profile.tracking, before.tracking);

    // `look_at: null` で外し、他は変えない。
    let mut with_look_at = model.clone();
    with_look_at.avatar = Some(profile.clone());
    let cleared = applied(
        &with_look_at,
        with_update(|c| {
            if let Command::UpdateAvatarProfile { look_at, .. } = c {
                *look_at = Some(None);
            }
        }),
    );
    let cleared = cleared.avatar.unwrap();
    assert_eq!(cleared.look_at, None);
    assert_eq!(cleared.meta, meta);
}

#[test]
fn update_rejects_invalid_values_without_creating_profile() {
    let model = fixtures::single_quad();
    let err = rejected(
        &model,
        with_update(|c| {
            if let Command::UpdateAvatarProfile { level, .. } = c {
                *level = Some(9);
            }
        }),
    );
    assert!(matches!(err, EditError::InvalidAvatarProfile { .. }));
    let err = rejected(
        &model,
        with_update(|c| {
            if let Command::UpdateAvatarProfile { tracking: t, .. } = c {
                *t = Some(vec![tracking(f32::INFINITY)]);
            }
        }),
    );
    assert!(matches!(err, EditError::InvalidAvatarProfile { .. }));
}

#[test]
fn update_look_at_json_distinguishes_null_from_missing() {
    let missing: Command = serde_json::from_str(r#"{"op":"update_avatar_profile"}"#).unwrap();
    assert_eq!(missing, update());
    let null: Command =
        serde_json::from_str(r#"{"op":"update_avatar_profile","look_at":null}"#).unwrap();
    let Command::UpdateAvatarProfile { look_at, .. } = &null else {
        panic!("unexpected command {null:?}");
    };
    assert_eq!(look_at, &Some(None));
    let json = serde_json::to_value(&null).unwrap();
    assert!(json["look_at"].is_null() && json.get("look_at").is_some());
    assert_eq!(serde_json::from_value::<Command>(json).unwrap(), null);
}

// ---- SetHitAreas と連鎖 ----

#[test]
fn set_hit_areas_replaces_list_and_forgets_removed_mappings() {
    let mut model = fixtures::avatar_profile();
    head_hit_area(&mut model);
    if let Some(avatar) = &mut model.avatar {
        avatar.hit_areas = vec![head_mapping()];
    }
    let body = vec![HitArea {
        id: "BodyArea".into(),
        mesh: "Quad".into(),
    }];
    let after = applied(
        &model,
        Command::SetHitAreas {
            hit_areas: body.clone(),
        },
    );
    assert_eq!(after.hit_areas, body);
    assert!(after.avatar.unwrap().hit_areas.is_empty());

    // 残った当たり判定への対応は保つ。
    let mut both = model.hit_areas.clone();
    both.extend(body);
    let kept = applied(&model, Command::SetHitAreas { hit_areas: both });
    assert_eq!(kept.avatar.unwrap().hit_areas, vec![head_mapping()]);
}

#[test]
fn set_hit_areas_rejects_bad_ids_and_meshes() {
    let model = fixtures::avatar_profile();
    let area = |id: &str, mesh: &str| HitArea {
        id: id.into(),
        mesh: mesh.into(),
    };
    let err = rejected(
        &model,
        Command::SetHitAreas {
            hit_areas: vec![area(" ", "Quad")],
        },
    );
    assert!(matches!(err, EditError::InvalidAvatarProfile { .. }));
    for hit_areas in [
        vec![area("A", "Missing")],
        vec![area("A", "Quad"), area("A", "Quad")],
    ] {
        let err = rejected(&model, Command::SetHitAreas { hit_areas });
        assert!(matches!(err, EditError::Invalid(_)), "{err:?}");
    }
}

#[test]
fn remove_mesh_forgets_standard_hit_areas() {
    let mut model = fixtures::avatar_profile();
    head_hit_area(&mut model);
    if let Some(avatar) = &mut model.avatar {
        avatar.hit_areas = vec![head_mapping()];
    }
    let after = applied(&model, Command::RemoveMesh { id: "Quad".into() });
    assert!(after.hit_areas.is_empty());
    assert!(after.avatar.unwrap().hit_areas.is_empty());
}

// ---- 準拠の検査 ----

#[test]
fn check_reports_levels_errors_and_warnings() {
    let model = fixtures::avatar_profile();
    let check = avatar::check(&model);
    assert_eq!(check.claimed, Some(3));
    assert_eq!(check.satisfied, 3);
    assert!(check.errors.is_empty(), "{:?}", check.errors);

    let none = avatar::check(&fixtures::single_quad());
    assert_eq!(none.claimed, None);
    assert_eq!(none.satisfied, 0);
    assert_eq!(none.errors.len(), 1);

    let mut unnamed = model.clone();
    if let Some(avatar) = &mut unnamed.avatar {
        avatar.meta.name.clear();
        avatar.profile_version = 99;
    }
    let check = avatar::check(&unnamed);
    assert_eq!(check.satisfied, 0);
    assert!(check.errors.iter().any(|e| e.message.contains("meta.name")));
    assert!(
        check
            .warnings
            .iter()
            .any(|w| w.message.contains("profile_version"))
    );
    // 画面はコードと引数で訳す (ADR 0029)。
    let missing = check
        .errors
        .iter()
        .find(|e| e.code == "avatar.missing_metadata")
        .unwrap();
    assert_eq!(missing.args["field"], MessageArg::Text("name".into()));
    let newer = check
        .warnings
        .iter()
        .find(|w| w.code == "avatar.newer_profile_version")
        .unwrap();
    assert_eq!(newer.args["version"], MessageArg::Number(99.0));
}
