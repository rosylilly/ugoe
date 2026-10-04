//! アバタープロファイルのデータ (`Model::avatar`)。仕様は `docs/spec/avatar-profile.md`、
//! 判断の経緯は ADR 0012。
//!
//! 準拠の検査は [`check`] ([`check::check_profile`])。トラッキングの値からパラメータを計算する処理は
//! このクレートには置かない (`docs/spec/avatar-profile.md#フェイストラッキングとの対応`)。

pub mod check;

use serde::{Deserialize, Serialize};

use crate::{HitAreaId, ParamId, SlotId, standard};

/// このクレートが知っているプロファイルのバージョン (`AvatarProfile::profile_version`)。
pub const PROFILE_VERSION: u32 = 0;

/// このクレートが知っている最も高い準拠レベル。
pub const MAX_LEVEL: u8 = 3;

/// レベル 1 (基本) に必要なパラメータ。
pub const LEVEL_1_PARAMETERS: &[&str] = &[
    standard::ANGLE_X,
    standard::ANGLE_Y,
    standard::ANGLE_Z,
    standard::EYE_L_OPEN,
    standard::EYE_R_OPEN,
    standard::MOUTH_OPEN_Y,
];

/// レベル 2 (標準) で、レベル 1 に加えて必要なパラメータ。
pub const LEVEL_2_PARAMETERS: &[&str] = &[
    standard::EYE_BALL_X,
    standard::EYE_BALL_Y,
    standard::BROW_L_Y,
    standard::BROW_R_Y,
    standard::MOUTH_FORM,
    standard::BODY_ANGLE_X,
    standard::BODY_ANGLE_Y,
    standard::BODY_ANGLE_Z,
    standard::BREATH,
];

/// レベル 3 (表情) で、レベル 2 に加えて必要な拡張パラメータ ([`standard::EXTENDED`]) の数。
pub const LEVEL_3_EXTENDED_COUNT: usize = 8;

/// 頭の左右の回転の入力 (度)。頭の回転の入力は [`TrackingSource::Head`] の `input` に書く。
pub const HEAD_YAW: &str = "yaw";
/// 頭の上下の回転の入力 (度)。
pub const HEAD_PITCH: &str = "pitch";
/// 頭の傾きの入力 (度)。
pub const HEAD_ROLL: &str = "roll";
/// 頭の回転の入力の一覧。
pub const HEAD_INPUTS: &[&str] = &[HEAD_YAW, HEAD_PITCH, HEAD_ROLL];

/// 頭の標準の当たり判定の名前 (`AvatarHitArea::name`)。
pub const HIT_AREA_HEAD: &str = "Head";
/// 体の標準の当たり判定の名前 (`AvatarHitArea::name`)。
pub const HIT_AREA_BODY: &str = "Body";

/// トラッキングの標準の対応表で値が決まるパラメータ ([`TrackingSource::Standard`] の `input` に書ける ID)。
/// 仕様書の「頭と体」と「ARKit のブレンドシェイプ」の表と同じ順。
pub const TRACKED_PARAMETERS: &[&str] = &[
    standard::ANGLE_X,
    standard::ANGLE_Y,
    standard::ANGLE_Z,
    standard::BODY_ANGLE_X,
    standard::BODY_ANGLE_Y,
    standard::BODY_ANGLE_Z,
    standard::SHOULDER_L_Y,
    standard::SHOULDER_R_Y,
    standard::EYE_L_OPEN,
    standard::EYE_R_OPEN,
    standard::EYE_L_WIDE,
    standard::EYE_R_WIDE,
    standard::EYE_L_SMILE,
    standard::EYE_R_SMILE,
    standard::EYE_BALL_X,
    standard::EYE_BALL_Y,
    standard::BROW_L_Y,
    standard::BROW_R_Y,
    standard::BROW_INNER_UP,
    standard::BROW_L_ANGLE,
    standard::BROW_R_ANGLE,
    standard::MOUTH_OPEN_Y,
    standard::MOUTH_FORM,
    standard::MOUTH_X,
    standard::JAW_X,
    standard::MOUTH_PUCKER,
    standard::MOUTH_FUNNEL,
    standard::MOUTH_PRESS,
    standard::MOUTH_SHRUG,
    standard::CHEEK_PUFF,
    standard::TONGUE_OUT,
    standard::NOSE_SNEER,
];

/// ARKit の 52 ブレンドシェイプの名前 ([`TrackingSource::BlendShape`] の `input` に書く)。
pub const ARKIT_BLEND_SHAPES: [&str; 52] = [
    "eyeBlinkLeft",
    "eyeLookDownLeft",
    "eyeLookInLeft",
    "eyeLookOutLeft",
    "eyeLookUpLeft",
    "eyeSquintLeft",
    "eyeWideLeft",
    "eyeBlinkRight",
    "eyeLookDownRight",
    "eyeLookInRight",
    "eyeLookOutRight",
    "eyeLookUpRight",
    "eyeSquintRight",
    "eyeWideRight",
    "jawForward",
    "jawLeft",
    "jawRight",
    "jawOpen",
    "mouthClose",
    "mouthFunnel",
    "mouthPucker",
    "mouthLeft",
    "mouthRight",
    "mouthSmileLeft",
    "mouthSmileRight",
    "mouthFrownLeft",
    "mouthFrownRight",
    "mouthDimpleLeft",
    "mouthDimpleRight",
    "mouthStretchLeft",
    "mouthStretchRight",
    "mouthRollLower",
    "mouthRollUpper",
    "mouthShrugLower",
    "mouthShrugUpper",
    "mouthPressLeft",
    "mouthPressRight",
    "mouthLowerDownLeft",
    "mouthLowerDownRight",
    "mouthUpperUpLeft",
    "mouthUpperUpRight",
    "browDownLeft",
    "browDownRight",
    "browInnerUp",
    "browOuterUpLeft",
    "browOuterUpRight",
    "cheekPuff",
    "cheekSquintLeft",
    "cheekSquintRight",
    "noseSneerLeft",
    "noseSneerRight",
    "tongueOut",
];

/// レベル `level` に必要なパラメータ (そのレベル以下のすべて)。拡張パラメータの数の条件は含まない。
pub fn required_parameters(level: u8) -> impl Iterator<Item = &'static str> {
    let level_2: &[&str] = if level >= 2 { LEVEL_2_PARAMETERS } else { &[] };
    let level_1: &[&str] = if level >= 1 { LEVEL_1_PARAMETERS } else { &[] };
    level_1.iter().chain(level_2).copied()
}

/// アバタープロファイル (`Model::avatar`)。`docs/spec/avatar-profile.md`。
///
/// 省略したフィールドは既定値になる。許諾の既定値は最も制限の強い値。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct AvatarProfile {
    /// 準拠するプロファイルのバージョン。既定は [`PROFILE_VERSION`]。
    pub profile_version: u32,
    /// 準拠を主張するレベル (1〜[`MAX_LEVEL`])。既定は 1。
    pub level: u8,
    /// メタデータと利用許諾。
    pub meta: AvatarMeta,
    /// トラッキングの対応の上書きと追加。
    pub tracking: Vec<TrackingOverride>,
    /// 視線の追従。省略すると [`LookAt::default`] (`EyeBallX` / `EyeBallY`) を、モデルにあれば使う。
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub look_at: Option<LookAt>,
    /// 標準の当たり判定の名前と `Model::hit_areas` の対応。
    pub hit_areas: Vec<AvatarHitArea>,
    /// アプリに公開する着せ替えのスロットの ID (`docs/spec/modules.md`)。
    pub slots: Vec<SlotId>,
}

impl Default for AvatarProfile {
    fn default() -> Self {
        Self {
            profile_version: PROFILE_VERSION,
            level: 1,
            meta: AvatarMeta::default(),
            tracking: Vec::new(),
            look_at: None,
            hit_areas: Vec::new(),
            slots: Vec::new(),
        }
    }
}

/// アバターのメタデータと利用許諾 (`docs/spec/avatar-profile.md#メタデータ`)。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct AvatarMeta {
    /// アバターの名前 (必須)。
    pub name: String,
    /// アバターのバージョン。
    pub version: Option<String>,
    /// 作者の一覧 (必須。1 人以上)。
    pub authors: Vec<String>,
    /// 著作権の表示。
    pub copyright: Option<String>,
    /// 連絡先。
    pub contact: Option<String>,
    /// 原作・参考元。
    pub references: Vec<String>,
    /// サムネイル画像 (`Model::textures` のインデックス)。
    pub thumbnail: Option<u32>,
    /// 利用規約の URL。
    pub license_url: Option<String>,
    /// アバターとして使ってよい人。
    pub avatar_permission: AvatarPermission,
    /// 商用利用の範囲。
    pub commercial_usage: CommercialUsage,
    /// 過度に暴力的な表現での利用を許すか。
    pub allow_violent_usage: bool,
    /// 過度に性的な表現での利用を許すか。
    pub allow_sexual_usage: bool,
    /// 政治・宗教の目的での利用を許すか。
    pub allow_political_or_religious_usage: bool,
    /// 反社会的・差別的な目的での利用を許すか。
    pub allow_antisocial_or_hate_usage: bool,
    /// クレジット表記が要るか。
    pub credit_notation: CreditNotation,
    /// 改変していないモデル (ファイル) の再配布を許すか。
    pub allow_redistribution: bool,
    /// 改変の許諾。
    pub modification: Modification,
    /// 素材などの第三者のライセンス表記。
    pub third_party_licenses: Option<String>,
}

/// アバターとして使ってよい人。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum AvatarPermission {
    /// 作者だけ。
    #[default]
    OnlyAuthor,
    /// 作者が許諾した人だけ。
    OnlyLicensed,
    /// 誰でも。
    Everyone,
    /// 未知の値 (新しい版のファイル)。最も制限の強い `OnlyAuthor` として扱う。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl AvatarPermission {
    /// フォールバックを適用した値 (`Unknown` は `OnlyAuthor`)。
    pub fn effective(self) -> Self {
        match self {
            Self::Unknown => Self::OnlyAuthor,
            value => value,
        }
    }
}

/// 商用利用の範囲。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CommercialUsage {
    /// 個人の非営利の利用だけ。
    #[default]
    PersonalNonProfit,
    /// 個人の営利の利用まで。
    PersonalProfit,
    /// 法人の利用まで。
    Corporation,
    /// 未知の値 (新しい版のファイル)。最も制限の強い `PersonalNonProfit` として扱う。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl CommercialUsage {
    /// フォールバックを適用した値 (`Unknown` は `PersonalNonProfit`)。
    pub fn effective(self) -> Self {
        match self {
            Self::Unknown => Self::PersonalNonProfit,
            value => value,
        }
    }
}

/// クレジット表記。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CreditNotation {
    /// クレジットの表記が要る。
    #[default]
    Required,
    /// クレジットの表記は要らない。
    Unnecessary,
    /// 未知の値 (新しい版のファイル)。最も制限の強い `Required` として扱う。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl CreditNotation {
    /// フォールバックを適用した値 (`Unknown` は `Required`)。
    pub fn effective(self) -> Self {
        match self {
            Self::Unknown => Self::Required,
            value => value,
        }
    }
}

/// 改変の許諾。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Modification {
    /// 改変を禁止する。
    #[default]
    Prohibited,
    /// 改変してよい (改変したものの再配布は不可)。
    Allow,
    /// 改変し、改変したものを再配布してよい。
    AllowRedistribution,
    /// 未知の値 (新しい版のファイル)。最も制限の強い `Prohibited` として扱う。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl Modification {
    /// フォールバックを適用した値 (`Unknown` は `Prohibited`)。
    pub fn effective(self) -> Self {
        match self {
            Self::Unknown => Self::Prohibited,
            value => value,
        }
    }
}

/// トラッキングの対応の上書き 1 つ (`docs/spec/avatar-profile.md#トラッキングの対応の上書き`)。
///
/// 入力の値 `v` から `v * scale + offset` を `parameter` に書き込む (値域へのクランプはその後)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TrackingOverride {
    /// 入力の種類。`input` の名前の解釈を決める。
    pub source: TrackingSource,
    /// 入力の名前。`source` によって、標準の対応表のパラメータ ID ([`TRACKED_PARAMETERS`])、
    /// ARKit のブレンドシェイプ名 ([`ARKIT_BLEND_SHAPES`])、頭の回転 ([`HEAD_INPUTS`]) のいずれか。
    pub input: String,
    /// 書き込むパラメータ。`source` が `standard` なら省略でき、省略すると `input` と同じ ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter: Option<ParamId>,
    /// 入力の値に掛ける倍率。既定は 1。
    #[serde(default = "one")]
    pub scale: f32,
    /// 倍率を掛けた後に足す値。既定は 0。
    #[serde(default)]
    pub offset: f32,
}

impl TrackingOverride {
    /// 書き込むパラメータの ID (`parameter` を省略した `standard` の上書きでは `input`)。
    /// `standard` 以外で `parameter` が無ければ `None`。
    pub fn target(&self) -> Option<&str> {
        match (&self.parameter, self.source) {
            (Some(parameter), _) => Some(parameter.as_str()),
            (None, TrackingSource::Standard) => Some(&self.input),
            (None, _) => None,
        }
    }
}

fn one() -> f32 {
    1.0
}

/// トラッキングの入力の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TrackingSource {
    /// 標準の対応表で計算した、パラメータ `input` の値 (値域へのクランプ前)。
    Standard,
    /// ARKit のブレンドシェイプ `input` の値 (0〜1)。
    BlendShape,
    /// 頭の回転 `input` (`yaw` / `pitch` / `roll`、度)。
    Head,
    /// 未知の値 (新しい版のファイル)。その上書きを無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

/// 視線の追従に使うパラメータと可動範囲。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct LookAt {
    /// 左右 (正でキャラクターの左)。既定は `EyeBallX`。
    pub x: ParamId,
    /// 上下 (正で上)。既定は `EyeBallY`。
    pub y: ParamId,
    /// 左右の可動範囲 (度)。見る先がこの角度だけ横にあるとき、`x` が端の値になる。0 より大きい。
    pub range_x: f32,
    /// 上下の可動範囲 (度)。0 より大きい。
    pub range_y: f32,
}

impl Default for LookAt {
    fn default() -> Self {
        Self {
            x: standard::EYE_BALL_X.into(),
            y: standard::EYE_BALL_Y.into(),
            range_x: 30.0,
            range_y: 30.0,
        }
    }
}

/// 標準の当たり判定の名前と `HitArea` の対応。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AvatarHitArea {
    /// 標準の名前 ([`HIT_AREA_HEAD`]、[`HIT_AREA_BODY`])。
    pub name: String,
    /// `Model::hit_areas` の ID。
    pub hit_area: HitAreaId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_default_to_most_restrictive() {
        let meta: AvatarMeta = serde_json::from_str(r#"{"name":"A","authors":["B"]}"#).unwrap();
        assert_eq!(meta.avatar_permission, AvatarPermission::OnlyAuthor);
        assert_eq!(meta.commercial_usage, CommercialUsage::PersonalNonProfit);
        assert!(!meta.allow_violent_usage);
        assert!(!meta.allow_sexual_usage);
        assert!(!meta.allow_political_or_religious_usage);
        assert!(!meta.allow_antisocial_or_hate_usage);
        assert_eq!(meta.credit_notation, CreditNotation::Required);
        assert!(!meta.allow_redistribution);
        assert_eq!(meta.modification, Modification::Prohibited);
    }

    #[test]
    fn unknown_permissions_fall_back_to_most_restrictive() {
        let meta: AvatarMeta = serde_json::from_str(
            r#"{"avatar_permission":"x","commercial_usage":"x","credit_notation":"x",
                "modification":"x"}"#,
        )
        .unwrap();
        assert_eq!(meta.avatar_permission, AvatarPermission::Unknown);
        assert_eq!(
            meta.avatar_permission.effective(),
            AvatarPermission::OnlyAuthor
        );
        assert_eq!(
            meta.commercial_usage.effective(),
            CommercialUsage::PersonalNonProfit
        );
        assert_eq!(meta.credit_notation.effective(), CreditNotation::Required);
        assert_eq!(meta.modification.effective(), Modification::Prohibited);
        assert!(serde_json::to_string(&meta).is_err());
    }

    #[test]
    fn missing_fields_use_defaults() {
        let profile: AvatarProfile = serde_json::from_str("{}").unwrap();
        assert_eq!(profile, AvatarProfile::default());
        assert_eq!(profile.profile_version, PROFILE_VERSION);
        assert_eq!(profile.level, 1);
        assert_eq!(profile.look_at, None);
        assert_eq!(LookAt::default().x.as_str(), standard::EYE_BALL_X);
    }

    #[test]
    fn tracking_override_reads_defaults_and_resolves_target() {
        let list: Vec<TrackingOverride> = serde_json::from_str(
            r#"[{"source":"standard","input":"EyeBallX","scale":-1},
                {"source":"blend_shape","input":"jawForward","parameter":"JawForward"},
                {"source":"head","input":"yaw"},
                {"source":"body","input":"yaw"}]"#,
        )
        .unwrap();
        assert_eq!(list[0].scale, -1.0);
        assert_eq!(list[0].target(), Some("EyeBallX"));
        assert_eq!(list[1].scale, 1.0);
        assert_eq!(list[1].offset, 0.0);
        assert_eq!(list[1].target(), Some("JawForward"));
        assert_eq!(list[2].target(), None);
        assert_eq!(list[3].source, TrackingSource::Unknown);
    }

    #[test]
    fn profile_roundtrips_json() {
        let profile = AvatarProfile {
            level: 2,
            tracking: vec![TrackingOverride {
                source: TrackingSource::BlendShape,
                input: "cheekSquintLeft".into(),
                parameter: Some("CheekL".into()),
                scale: 0.5,
                offset: 0.1,
            }],
            hit_areas: vec![AvatarHitArea {
                name: HIT_AREA_HEAD.into(),
                hit_area: "HeadArea".into(),
            }],
            ..AvatarProfile::default()
        };
        let json = serde_json::to_string(&profile).unwrap();
        let back: AvatarProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, back);
    }

    #[test]
    fn tables_use_known_names() {
        for id in required_parameters(MAX_LEVEL).chain(TRACKED_PARAMETERS.iter().copied()) {
            assert!(standard::range(id).is_some(), "{id}");
        }
        assert_eq!(required_parameters(1).count(), LEVEL_1_PARAMETERS.len());
        assert_eq!(required_parameters(0).count(), 0);
        let mut shapes = ARKIT_BLEND_SHAPES.to_vec();
        shapes.sort_unstable();
        shapes.dedup();
        assert_eq!(shapes.len(), 52);
        // 拡張パラメータはすべて標準の対応表で値が決まる。
        for (id, ..) in standard::EXTENDED.iter().chain(standard::BODY_EXTENDED) {
            assert!(TRACKED_PARAMETERS.contains(id), "{id}");
        }
    }
}
