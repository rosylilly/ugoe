//! アバタープロファイル (`Model::avatar`) と当たり判定 (`Model::hit_areas`) の編集と、準拠の検査の結果。
//! 設計は `docs/editor.md#アバタープロファイルの編集`、データは `docs/spec/avatar-profile.md`。
//!
//! コマンドが拒否するのは、モデルによらず不正な値 (範囲外のレベル・有限でない数・書き出せない未知の値など)
//! だけ。パラメータ・当たり判定・スロット・テクスチャへの参照は、要素を作る順に縛られないように拒否せず、
//! 準拠の検査 ([`check`]) がエラーとして示す。

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_model::avatar::check::{Severity, check_profile};
use ugoe_model::avatar::{
    AvatarHitArea, AvatarMeta, AvatarPermission, AvatarProfile, CommercialUsage, CreditNotation,
    LookAt, MAX_LEVEL, Modification, TrackingOverride, TrackingSource,
};
use ugoe_model::{HitArea, Model, SlotId};

use crate::EditError;
use crate::message::{Coded, Message};

/// [`crate::Command::UpdateAvatarProfile`] の変更。`None` のフィールドは変えない。
#[derive(Debug, Default)]
pub(crate) struct ProfileUpdate {
    pub level: Option<u8>,
    pub meta: Option<AvatarMeta>,
    pub tracking: Option<Vec<TrackingOverride>>,
    /// `Some(None)` で外す (既定の `EyeBallX` / `EyeBallY` に戻す)。
    pub look_at: Option<Option<LookAt>>,
    pub hit_areas: Option<Vec<AvatarHitArea>>,
    pub slots: Option<Vec<SlotId>>,
}

/// [`crate::Command::SetAvatarProfile`]。
pub(crate) fn set_profile(
    model: &mut Model,
    profile: Option<AvatarProfile>,
) -> Result<(), EditError> {
    if let Some(profile) = &profile {
        check_level(profile.level)?;
        check_meta(&profile.meta)?;
        check_tracking(&profile.tracking)?;
        if let Some(look_at) = &profile.look_at {
            check_look_at(look_at)?;
        }
        check_hit_area_names(&profile.hit_areas)?;
    }
    model.avatar = profile;
    Ok(())
}

/// [`crate::Command::UpdateAvatarProfile`]。プロファイルがなければ既定値から作る。
pub(crate) fn update_profile(model: &mut Model, update: ProfileUpdate) -> Result<(), EditError> {
    let profile = model.avatar.get_or_insert_with(AvatarProfile::default);
    if let Some(level) = update.level {
        check_level(level)?;
        profile.level = level;
    }
    if let Some(meta) = update.meta {
        check_meta(&meta)?;
        profile.meta = meta;
    }
    if let Some(tracking) = update.tracking {
        check_tracking(&tracking)?;
        profile.tracking = tracking;
    }
    if let Some(look_at) = update.look_at {
        if let Some(look_at) = &look_at {
            check_look_at(look_at)?;
        }
        profile.look_at = look_at;
    }
    if let Some(hit_areas) = update.hit_areas {
        check_hit_area_names(&hit_areas)?;
        profile.hit_areas = hit_areas;
    }
    if let Some(slots) = update.slots {
        profile.slots = slots;
    }
    Ok(())
}

/// [`crate::Command::SetHitAreas`]。ID の重複とメッシュの参照は適用後の検証 (`validate`) が調べる。
/// なくなった当たり判定を指すプロファイルの標準の当たり判定は外す。
pub(crate) fn set_hit_areas(model: &mut Model, hit_areas: Vec<HitArea>) -> Result<(), EditError> {
    if let Some(empty) = hit_areas.iter().find(|h| h.id.as_str().trim().is_empty()) {
        return Err(invalid(format!(
            "hit area id must not be empty (mesh `{}`)",
            empty.mesh
        )));
    }
    model.hit_areas = hit_areas;
    forget_removed_hit_areas(model);
    Ok(())
}

/// `Model::hit_areas` にない当たり判定を指す、プロファイルの標準の当たり判定を外す
/// (`remove_mesh` と `set_hit_areas` の連鎖)。
pub(crate) fn forget_removed_hit_areas(model: &mut Model) {
    let Some(avatar) = &mut model.avatar else {
        return;
    };
    avatar
        .hit_areas
        .retain(|a| model.hit_areas.iter().any(|h| h.id == a.hit_area));
}

fn invalid(reason: String) -> EditError {
    EditError::InvalidAvatarProfile { reason }
}

fn check_level(level: u8) -> Result<(), EditError> {
    if (1..=MAX_LEVEL).contains(&level) {
        Ok(())
    } else {
        Err(invalid(format!(
            "level {level} must be between 1 and {MAX_LEVEL}"
        )))
    }
}

/// 未知の許諾の値は書き出せないので拒否する (`docs/spec/file-formats.md#未知の値の読み込み`)。
fn check_meta(meta: &AvatarMeta) -> Result<(), EditError> {
    let unknown = [
        (
            "avatar_permission",
            meta.avatar_permission == AvatarPermission::Unknown,
        ),
        (
            "commercial_usage",
            meta.commercial_usage == CommercialUsage::Unknown,
        ),
        (
            "credit_notation",
            meta.credit_notation == CreditNotation::Unknown,
        ),
        ("modification", meta.modification == Modification::Unknown),
    ];
    match unknown.iter().find(|(_, is_unknown)| *is_unknown) {
        Some((field, _)) => Err(invalid(format!("meta.{field} has an unknown value"))),
        None => Ok(()),
    }
}

fn check_tracking(tracking: &[TrackingOverride]) -> Result<(), EditError> {
    for (i, t) in tracking.iter().enumerate() {
        if t.source == TrackingSource::Unknown {
            return Err(invalid(format!(
                "tracking[{i}].source has an unknown value"
            )));
        }
        if !t.scale.is_finite() || !t.offset.is_finite() {
            return Err(invalid(format!(
                "tracking[{i}]: scale {} and offset {} must be finite",
                t.scale, t.offset
            )));
        }
    }
    Ok(())
}

fn check_look_at(look_at: &LookAt) -> Result<(), EditError> {
    for (field, range) in [("range_x", look_at.range_x), ("range_y", look_at.range_y)] {
        if !(range.is_finite() && range > 0.0) {
            return Err(invalid(format!(
                "look_at.{field} = {range} must be a finite value greater than 0"
            )));
        }
    }
    Ok(())
}

fn check_hit_area_names(hit_areas: &[AvatarHitArea]) -> Result<(), EditError> {
    for (i, a) in hit_areas.iter().enumerate() {
        if hit_areas[..i].iter().any(|b| b.name == a.name) {
            return Err(invalid(format!(
                "hit_areas: name `{}` appears more than once",
                a.name
            )));
        }
    }
    Ok(())
}

/// `null` と省略を区別する `Option<Option<T>>` の読み書き (`#[serde(default, with = ...)]` と組み合わせる)。
/// 省略は `None`、`null` は `Some(None)`、値は `Some(Some(_))`。
pub(crate) mod nullable {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S, T>(value: &Option<Option<T>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        value
            .as_ref()
            .and_then(Option::as_ref)
            .serialize(serializer)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Some)
    }
}

/// 準拠の検査 (`ugoe_model::avatar::check::check_profile`) の結果を、画面に出す形にしたもの。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct AvatarCheck {
    /// モデルが主張するレベル。プロファイルがなければ `None`。
    pub claimed: Option<u8>,
    /// メタデータと必要なパラメータから見て満たしているレベル (どのレベルも満たさなければ 0)。
    pub satisfied: u8,
    /// エラー (1 つでもあれば準拠していない)。コードは `avatar.*` (`docs/editor.md#エラーと警告のコード`)。
    pub errors: Vec<Message>,
    /// 警告 (準拠は妨げない)。コードは `avatar.*`。
    pub warnings: Vec<Message>,
}

/// モデルのアバタープロファイルへの準拠を調べる。モデルは変えない。
pub fn check(model: &Model) -> AvatarCheck {
    let report = check_profile(model);
    let messages = |severity| {
        report
            .issues
            .iter()
            .filter(|i| i.severity() == severity)
            .map(Coded::to_message)
            .collect()
    };
    AvatarCheck {
        claimed: report.claimed,
        satisfied: report.satisfied,
        errors: messages(Severity::Error),
        warnings: messages(Severity::Warning),
    }
}
