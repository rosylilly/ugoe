//! アバタープロファイルへの準拠の検査 (`ugoe validate --profile avatar`)。
//! 検査項目は `docs/spec/avatar-profile.md#準拠の検査`。
//!
//! 構造の検証 ([`crate::validate::validate`]) とは別に呼ぶ。`avatar` はランタイムが読み飛ばせるデータ
//! (機能 `avatar.profile`) なので、ここで見つけた問題はモデルの読み込みを拒否する理由にしない。

use std::collections::HashSet;

use super::{
    ARKIT_BLEND_SHAPES, AvatarProfile, HEAD_INPUTS, LEVEL_1_PARAMETERS, LEVEL_2_PARAMETERS,
    LEVEL_3_EXTENDED_COUNT, MAX_LEVEL, PROFILE_VERSION, TRACKED_PARAMETERS, TrackingSource,
};
use crate::{Model, standard};

/// 問題の重さ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// 準拠していない。
    Error,
    /// 準拠はしているが、アプリによっては期待どおりに動かない可能性がある。
    Warning,
}

/// 準拠の検査で見つけた問題。`field` は値の場所 (`avatar.tracking[0].parameter` など)。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProfileIssue {
    #[error("model has no avatar profile (`avatar`)")]
    MissingProfile,
    #[error(
        "avatar.profile_version {version} is newer than the supported version {supported}; \
         the checks may be incomplete"
    )]
    NewerProfileVersion { version: u32, supported: u32 },
    #[error("avatar.level {level} must be between 1 and {max}")]
    InvalidLevel { level: u8, max: u8 },
    #[error("avatar.meta.{field} is required")]
    MissingMetadata { field: &'static str },
    #[error("level {level} requires parameter `{id}`")]
    MissingParameter { level: u8, id: &'static str },
    #[error("level 3 requires at least {required} extended parameters, found {found}")]
    TooFewExtendedParameters { required: usize, found: usize },
    /// 標準・拡張パラメータの値域が仕様と違う。
    #[error(
        "parameter `{id}`: range {min} / {default} / {max} differs from the profile's \
         {} / {} / {}", expected.0, expected.1, expected.2
    )]
    ParameterRange {
        id: String,
        min: f32,
        default: f32,
        max: f32,
        /// 仕様の `(min, default, max)`。
        expected: (f32, f32, f32),
    },
    #[error("{field} refers to unknown {target_kind} `{target}`")]
    UnknownReference {
        field: String,
        target_kind: &'static str,
        target: String,
    },
    /// トラッキングの上書きの `input` が、`source` の種類の入力として知られていない。
    #[error("{field}: `{input}` is not a known {kind} input")]
    UnknownTrackingInput {
        field: String,
        kind: &'static str,
        input: String,
    },
    /// `standard` 以外のトラッキングの上書きに `parameter` が無い。
    #[error("{field}: `parameter` is required unless the source is `standard`")]
    MissingTrackingTarget { field: String },
    #[error("avatar.hit_areas: name `{name}` appears more than once")]
    DuplicateHitAreaName { name: String },
    /// 新しい版のファイルの未知の値。フォールバックを適用して扱う。
    #[error("{field} has an unknown value from a newer file; its fallback is used")]
    UnknownValue { field: String },
    #[error("{field} = {value} must be {expected}")]
    InvalidValue {
        field: String,
        value: f32,
        expected: &'static str,
    },
    /// トラッキング・視線の対象が中継用 (`internal`) のパラメータ。トラッキングはこのパラメータに書き込まない
    /// (`docs/spec/model.md#パラメータ`)。
    #[error("{field}: parameter `{id}` is internal, so tracking does not write it")]
    InternalTrackingTarget { field: String, id: String },
}

impl ProfileIssue {
    /// 問題の重さ。
    pub fn severity(&self) -> Severity {
        match self {
            Self::NewerProfileVersion { .. }
            | Self::ParameterRange { .. }
            | Self::UnknownValue { .. }
            | Self::InternalTrackingTarget { .. } => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

/// 準拠の検査の結果。
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileReport {
    /// モデルが主張するレベル (`avatar.level`)。`avatar` が無ければ `None`。
    pub claimed: Option<u8>,
    /// メタデータと必要なパラメータから見て満たしているレベル。どのレベルも満たさなければ 0。
    /// 主張したレベルとは無関係に、[`MAX_LEVEL`] まで調べる。
    pub satisfied: u8,
    /// 見つけた問題 (エラーと警告)。
    pub issues: Vec<ProfileIssue>,
}

impl ProfileReport {
    /// エラーが無いか (警告は準拠を妨げない)。
    pub fn is_conformant(&self) -> bool {
        self.errors().next().is_none()
    }

    /// エラーだけを返す。
    pub fn errors(&self) -> impl Iterator<Item = &ProfileIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity() == Severity::Error)
    }

    /// 警告だけを返す。
    pub fn warnings(&self) -> impl Iterator<Item = &ProfileIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity() == Severity::Warning)
    }
}

/// モデルがアバタープロファイルに準拠しているかを調べる。
///
/// 主張したレベル (`avatar.level`) までに必要なパラメータとメタデータ、`avatar` の中の参照を調べる。
/// モデルが構造の検証 ([`crate::validate::validate`]) を通ることは前提にしない。
pub fn check_profile(model: &Model) -> ProfileReport {
    let has_param = |id: &str| model.parameters.iter().any(|p| p.id.as_str() == id);
    let extended = standard::EXTENDED
        .iter()
        .filter(|(id, ..)| has_param(id))
        .count();

    let Some(avatar) = &model.avatar else {
        return ProfileReport {
            claimed: None,
            satisfied: 0,
            issues: vec![ProfileIssue::MissingProfile],
        };
    };

    let mut issues = Vec::new();
    if avatar.profile_version > PROFILE_VERSION {
        issues.push(ProfileIssue::NewerProfileVersion {
            version: avatar.profile_version,
            supported: PROFILE_VERSION,
        });
    }
    if !(1..=MAX_LEVEL).contains(&avatar.level) {
        issues.push(ProfileIssue::InvalidLevel {
            level: avatar.level,
            max: MAX_LEVEL,
        });
    }

    let meta_ok = check_meta(model, avatar, &mut issues);

    // レベルごとの必要なパラメータ。主張したレベルまでは不足をエラーにする。
    let claimed = avatar.level.min(MAX_LEVEL);
    let levels: [(u8, &[&'static str]); 2] = [(1, LEVEL_1_PARAMETERS), (2, LEVEL_2_PARAMETERS)];
    let mut satisfied = if meta_ok { MAX_LEVEL } else { 0 };
    for (level, required) in levels {
        for &id in required {
            if !has_param(id) {
                satisfied = satisfied.min(level - 1);
                if level <= claimed {
                    issues.push(ProfileIssue::MissingParameter { level, id });
                }
            }
        }
    }
    if extended < LEVEL_3_EXTENDED_COUNT {
        satisfied = satisfied.min(2);
        if claimed >= 3 {
            issues.push(ProfileIssue::TooFewExtendedParameters {
                required: LEVEL_3_EXTENDED_COUNT,
                found: extended,
            });
        }
    }

    for param in &model.parameters {
        if param.internal && TRACKED_PARAMETERS.contains(&param.id.as_str()) {
            issues.push(ProfileIssue::InternalTrackingTarget {
                field: format!("parameters[{}].internal", param.id),
                id: param.id.to_string(),
            });
        }
        let Some(expected) = standard::range(param.id.as_str()) else {
            continue;
        };
        if (param.min, param.default, param.max) != expected {
            issues.push(ProfileIssue::ParameterRange {
                id: param.id.to_string(),
                min: param.min,
                default: param.default,
                max: param.max,
                expected,
            });
        }
    }

    check_tracking(model, avatar, &mut issues);
    check_look_at(model, avatar, &mut issues);
    check_hit_areas(model, avatar, &mut issues);
    check_slots(model, avatar, &mut issues);
    issues.extend(
        crate::unknown::in_model(model)
            .into_iter()
            .filter(|field| field.starts_with("avatar."))
            .map(|field| ProfileIssue::UnknownValue { field }),
    );

    ProfileReport {
        claimed: Some(avatar.level),
        satisfied,
        issues,
    }
}

/// メタデータの検査。必須のフィールドが揃っていれば `true` を返す。
fn check_meta(model: &Model, avatar: &AvatarProfile, issues: &mut Vec<ProfileIssue>) -> bool {
    let meta = &avatar.meta;
    let mut ok = true;
    if meta.name.trim().is_empty() {
        issues.push(ProfileIssue::MissingMetadata { field: "name" });
        ok = false;
    }
    if !meta.authors.iter().any(|a| !a.trim().is_empty()) {
        issues.push(ProfileIssue::MissingMetadata { field: "authors" });
        ok = false;
    }
    if let Some(thumbnail) = meta.thumbnail
        && usize::try_from(thumbnail).map_or(true, |t| t >= model.textures.len())
    {
        issues.push(ProfileIssue::UnknownReference {
            field: "avatar.meta.thumbnail".into(),
            target_kind: "texture",
            target: thumbnail.to_string(),
        });
    }
    ok
}

/// トラッキングの上書きの検査: 入力の名前、書き込むパラメータ、係数とオフセット。
fn check_tracking(model: &Model, avatar: &AvatarProfile, issues: &mut Vec<ProfileIssue>) {
    for (index, entry) in avatar.tracking.iter().enumerate() {
        let path = format!("avatar.tracking[{index}]");
        let (kind, known): (&str, &[&str]) = match entry.source {
            TrackingSource::Standard => ("standard", TRACKED_PARAMETERS),
            TrackingSource::BlendShape => ("blend_shape", &ARKIT_BLEND_SHAPES),
            TrackingSource::Head => ("head", HEAD_INPUTS),
            // 未知の入力の上書きは無視されるので検査しない (未知の値として下で報告する)。
            TrackingSource::Unknown => continue,
        };
        if !known.contains(&entry.input.as_str()) {
            issues.push(ProfileIssue::UnknownTrackingInput {
                field: format!("{path}.input"),
                kind,
                input: entry.input.clone(),
            });
        }
        match entry.target() {
            Some(target) => {
                let field = format!("{path}.parameter");
                check_param(model, &field, target, issues);
                check_not_internal(model, field, target, issues);
            }
            None => issues.push(ProfileIssue::MissingTrackingTarget {
                field: path.clone(),
            }),
        }
        check_finite(&format!("{path}.scale"), entry.scale, issues);
        check_finite(&format!("{path}.offset"), entry.offset, issues);
    }
}

/// 視線の検査: パラメータの参照と可動範囲。省略した場合 (標準の視線のパラメータを使う) は調べない。
fn check_look_at(model: &Model, avatar: &AvatarProfile, issues: &mut Vec<ProfileIssue>) {
    let Some(look_at) = &avatar.look_at else {
        return;
    };
    for (field, id) in [
        ("avatar.look_at.x", &look_at.x),
        ("avatar.look_at.y", &look_at.y),
    ] {
        check_param(model, field, id.as_str(), issues);
        check_not_internal(model, field.to_owned(), id.as_str(), issues);
    }
    let ranges = [("range_x", look_at.range_x), ("range_y", look_at.range_y)];
    for (name, value) in ranges {
        if !(value.is_finite() && value > 0.0) {
            issues.push(ProfileIssue::InvalidValue {
                field: format!("avatar.look_at.{name}"),
                value,
                expected: "finite and > 0",
            });
        }
    }
}

/// 当たり判定の対応の検査: 名前の重複と `HitArea` の参照。
fn check_hit_areas(model: &Model, avatar: &AvatarProfile, issues: &mut Vec<ProfileIssue>) {
    let mut seen = HashSet::new();
    let mut reported = HashSet::new();
    for (index, entry) in avatar.hit_areas.iter().enumerate() {
        if !seen.insert(&entry.name) && reported.insert(&entry.name) {
            issues.push(ProfileIssue::DuplicateHitAreaName {
                name: entry.name.clone(),
            });
        }
        if !model.hit_areas.iter().any(|h| h.id == entry.hit_area) {
            issues.push(ProfileIssue::UnknownReference {
                field: format!("avatar.hit_areas[{index}].hit_area"),
                target_kind: "hit area",
                target: entry.hit_area.to_string(),
            });
        }
    }
}

/// 公開するスロットが `Model::slots` にあること。
fn check_slots(model: &Model, avatar: &AvatarProfile, issues: &mut Vec<ProfileIssue>) {
    for (index, slot) in avatar.slots.iter().enumerate() {
        if !model.slots.iter().any(|s| s.id == *slot) {
            issues.push(ProfileIssue::UnknownReference {
                field: format!("avatar.slots[{index}]"),
                target_kind: "slot",
                target: slot.to_string(),
            });
        }
    }
}

/// パラメータ `id` がモデルにあること。
fn check_param(model: &Model, field: &str, id: &str, issues: &mut Vec<ProfileIssue>) {
    if !model.parameters.iter().any(|p| p.id.as_str() == id) {
        issues.push(ProfileIssue::UnknownReference {
            field: field.to_owned(),
            target_kind: "parameter",
            target: id.to_owned(),
        });
    }
}

/// パラメータ `id` が中継用 (`internal`) でないこと (警告)。
fn check_not_internal(model: &Model, field: String, id: &str, issues: &mut Vec<ProfileIssue>) {
    if model
        .parameters
        .iter()
        .any(|p| p.internal && p.id.as_str() == id)
    {
        issues.push(ProfileIssue::InternalTrackingTarget {
            field,
            id: id.to_owned(),
        });
    }
}

/// 有限値であること。
fn check_finite(field: &str, value: f32, issues: &mut Vec<ProfileIssue>) {
    if !value.is_finite() {
        issues.push(ProfileIssue::InvalidValue {
            field: field.to_owned(),
            value,
            expected: "finite",
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avatar::{AvatarHitArea, AvatarMeta, LookAt, TrackingOverride};
    use crate::fixtures;
    use crate::parameter::{Parameter, ParameterKind};

    fn param(id: &str) -> Parameter {
        let (min, default, max) = standard::range(id).unwrap_or((0.0, 0.0, 1.0));
        Parameter {
            id: id.into(),
            name: id.into(),
            min,
            max,
            default,
            kind: ParameterKind::Normal,
            mirror: None,
            group: None,
            internal: false,
        }
    }

    /// レベル 1 のパラメータとメタデータだけを持つモデル。
    fn level_1() -> Model {
        let mut model = fixtures::single_quad();
        model
            .parameters
            .extend(LEVEL_1_PARAMETERS.iter().map(|id| param(id)));
        model.avatar = Some(AvatarProfile {
            meta: AvatarMeta {
                name: "A".into(),
                authors: vec!["B".into()],
                ..AvatarMeta::default()
            },
            ..AvatarProfile::default()
        });
        model
    }

    fn avatar(model: &mut Model) -> &mut AvatarProfile {
        model.avatar.as_mut().unwrap()
    }

    #[test]
    fn fixture_satisfies_level_3() {
        let report = check_profile(&fixtures::avatar_profile());
        assert_eq!(report.issues, vec![]);
        assert_eq!(report.claimed, Some(3));
        assert_eq!(report.satisfied, 3);
        assert!(report.is_conformant());
    }

    #[test]
    fn missing_profile_is_an_error() {
        let report = check_profile(&fixtures::single_quad());
        assert_eq!(report.claimed, None);
        assert_eq!(report.satisfied, 0);
        assert_eq!(report.issues, vec![ProfileIssue::MissingProfile]);
        assert!(!report.is_conformant());
    }

    #[test]
    fn level_1_model_satisfies_only_level_1() {
        let mut model = level_1();
        let report = check_profile(&model);
        assert_eq!(report.issues, vec![]);
        assert_eq!(report.satisfied, 1);

        // レベル 2 を主張すると、レベル 2 のパラメータの不足がエラーになる。
        avatar(&mut model).level = 2;
        let report = check_profile(&model);
        assert_eq!(report.errors().count(), LEVEL_2_PARAMETERS.len());
        assert!(report.issues.contains(&ProfileIssue::MissingParameter {
            level: 2,
            id: standard::BREATH,
        }));
        assert_eq!(report.satisfied, 1);
    }

    #[test]
    fn level_3_counts_extended_parameters() {
        let mut model = level_1();
        model
            .parameters
            .extend(LEVEL_2_PARAMETERS.iter().map(|id| param(id)));
        model
            .parameters
            .extend(standard::EXTENDED[..7].iter().map(|(id, ..)| param(id)));
        avatar(&mut model).level = 3;
        let report = check_profile(&model);
        assert_eq!(
            report.issues,
            vec![ProfileIssue::TooFewExtendedParameters {
                required: 8,
                found: 7
            }]
        );
        assert_eq!(report.satisfied, 2);

        // 体の拡張パラメータ (肩) はレベル 3 の数に入らない。
        model
            .parameters
            .extend(standard::BODY_EXTENDED.iter().map(|(id, ..)| param(id)));
        assert_eq!(check_profile(&model).satisfied, 2);

        model.parameters.push(param(standard::EXTENDED[7].0));
        let report = check_profile(&model);
        assert!(report.is_conformant());
        assert_eq!(report.satisfied, 3);
    }

    #[test]
    fn missing_metadata_fails_every_level() {
        let mut model = level_1();
        avatar(&mut model).meta = AvatarMeta {
            authors: vec![" ".into()],
            ..AvatarMeta::default()
        };
        let report = check_profile(&model);
        assert_eq!(
            report.issues,
            vec![
                ProfileIssue::MissingMetadata { field: "name" },
                ProfileIssue::MissingMetadata { field: "authors" },
            ]
        );
        assert_eq!(report.satisfied, 0);
    }

    #[test]
    fn invalid_and_newer_versions() {
        let mut model = level_1();
        avatar(&mut model).level = 0;
        avatar(&mut model).profile_version = PROFILE_VERSION + 1;
        let report = check_profile(&model);
        assert_eq!(
            report.issues,
            vec![
                ProfileIssue::NewerProfileVersion {
                    version: PROFILE_VERSION + 1,
                    supported: PROFILE_VERSION,
                },
                ProfileIssue::InvalidLevel { level: 0, max: 3 },
            ]
        );
        assert_eq!(report.warnings().count(), 1);
    }

    #[test]
    fn range_mismatch_is_a_warning() {
        let mut model = level_1();
        let angle = model
            .parameters
            .iter_mut()
            .find(|p| p.id.as_str() == standard::ANGLE_X)
            .unwrap();
        angle.min = -45.0;
        let report = check_profile(&model);
        assert!(report.is_conformant());
        assert!(matches!(
            report.issues.as_slice(),
            [ProfileIssue::ParameterRange { id, expected: (-30.0, 0.0, 30.0), .. }]
                if id == standard::ANGLE_X
        ));
    }

    #[test]
    fn tracking_overrides_are_checked() {
        let mut model = level_1();
        let entry = |source, input: &str, parameter: Option<&str>| TrackingOverride {
            source,
            input: input.into(),
            parameter: parameter.map(Into::into),
            scale: 1.0,
            offset: 0.0,
        };
        avatar(&mut model).tracking = vec![
            // 正しい: 標準の対応の係数だけを変える・ARKit の未使用のブレンドシェイプを足す。
            entry(TrackingSource::Standard, standard::ANGLE_X, None),
            entry(TrackingSource::BlendShape, "jawForward", Some("X")),
            entry(TrackingSource::Head, "roll", Some("X")),
            // 誤り。
            entry(TrackingSource::Standard, "Cheek", None),
            entry(TrackingSource::BlendShape, "jawOpenWide", None),
            entry(TrackingSource::Head, "yaw", Some("Nope")),
            TrackingOverride {
                scale: f32::NAN,
                ..entry(TrackingSource::Unknown, "x", None)
            },
        ];
        let report = check_profile(&model);
        assert_eq!(
            report.issues,
            vec![
                ProfileIssue::UnknownTrackingInput {
                    field: "avatar.tracking[3].input".into(),
                    kind: "standard",
                    input: "Cheek".into(),
                },
                ProfileIssue::UnknownReference {
                    field: "avatar.tracking[3].parameter".into(),
                    target_kind: "parameter",
                    target: "Cheek".into(),
                },
                ProfileIssue::UnknownTrackingInput {
                    field: "avatar.tracking[4].input".into(),
                    kind: "blend_shape",
                    input: "jawOpenWide".into(),
                },
                ProfileIssue::MissingTrackingTarget {
                    field: "avatar.tracking[4]".into(),
                },
                ProfileIssue::UnknownReference {
                    field: "avatar.tracking[5].parameter".into(),
                    target_kind: "parameter",
                    target: "Nope".into(),
                },
                ProfileIssue::UnknownValue {
                    field: "avatar.tracking[6].source".into(),
                },
            ]
        );
    }

    #[test]
    fn look_at_hit_areas_and_thumbnail_are_checked() {
        let mut model = level_1();
        model.hit_areas.push(crate::HitArea {
            id: "HeadArea".into(),
            mesh: "Quad".into(),
        });
        let profile = avatar(&mut model);
        profile.meta.thumbnail = Some(1);
        profile.look_at = Some(LookAt {
            range_y: 0.0,
            ..LookAt::default()
        });
        let hit = |name: &str, hit_area: &str| AvatarHitArea {
            name: name.into(),
            hit_area: hit_area.into(),
        };
        profile.hit_areas = vec![
            hit("Head", "HeadArea"),
            hit("Head", "HeadArea"),
            hit("Body", "BodyArea"),
        ];
        let report = check_profile(&model);
        assert_eq!(
            report.issues,
            vec![
                ProfileIssue::UnknownReference {
                    field: "avatar.meta.thumbnail".into(),
                    target_kind: "texture",
                    target: "1".into(),
                },
                // level_1 は EyeBallX / EyeBallY を持たない。
                ProfileIssue::UnknownReference {
                    field: "avatar.look_at.x".into(),
                    target_kind: "parameter",
                    target: "EyeBallX".into(),
                },
                ProfileIssue::UnknownReference {
                    field: "avatar.look_at.y".into(),
                    target_kind: "parameter",
                    target: "EyeBallY".into(),
                },
                ProfileIssue::InvalidValue {
                    field: "avatar.look_at.range_y".into(),
                    value: 0.0,
                    expected: "finite and > 0",
                },
                ProfileIssue::DuplicateHitAreaName {
                    name: "Head".into()
                },
                ProfileIssue::UnknownReference {
                    field: "avatar.hit_areas[2].hit_area".into(),
                    target_kind: "hit area",
                    target: "BodyArea".into(),
                },
            ]
        );
    }

    #[test]
    fn exposed_slots_must_exist() {
        let mut model = level_1();
        model.slots.push(crate::Slot {
            id: "Hair".into(),
            anchor: None,
            draw_order: crate::DrawOrderRange {
                min: 0.0,
                max: 1000.0,
            },
            replaces: Vec::new(),
            parameters: Vec::new(),
            mask_sources: Vec::new(),
            part: None,
            colliders: Vec::new(),
            pose_parts: Vec::new(),
        });
        avatar(&mut model).slots = vec!["Hair".into(), "Outfit".into()];
        assert_eq!(
            check_profile(&model).issues,
            vec![ProfileIssue::UnknownReference {
                field: "avatar.slots[1]".into(),
                target_kind: "slot",
                target: "Outfit".into(),
            }]
        );
    }

    /// 中継用 (`internal`) のパラメータはトラッキングが書き込まないので、トラッキングの対象なら警告する。
    #[test]
    fn internal_tracking_targets_are_warnings() {
        let mut model = level_1();
        model.parameters.push(Parameter {
            internal: true,
            ..param("Relay")
        });
        for p in &mut model.parameters {
            if p.id.as_str() == standard::ANGLE_X {
                p.internal = true;
            }
        }
        avatar(&mut model).tracking = vec![TrackingOverride {
            source: TrackingSource::BlendShape,
            input: "jawForward".into(),
            parameter: Some("Relay".into()),
            scale: 1.0,
            offset: 0.0,
        }];
        let report = check_profile(&model);
        assert_eq!(
            report.issues,
            vec![
                ProfileIssue::InternalTrackingTarget {
                    field: "parameters[AngleX].internal".into(),
                    id: standard::ANGLE_X.into(),
                },
                ProfileIssue::InternalTrackingTarget {
                    field: "avatar.tracking[0].parameter".into(),
                    id: "Relay".into(),
                },
            ]
        );
        assert!(report.is_conformant());
    }
}
