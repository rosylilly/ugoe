//! ugoe の **データ定義 (スキーマ) クレート**。
//!
//! ここにはシリアライズ可能な「モデルが何であるか」の定義だけを置き、
//! 「どう動かすか」(補間・変形・物理・モーション再生) は下流クレートが担う。
//! 仕様は `docs/spec/model.md` を正とする。型を変更したら仕様書も同じ PR で更新すること。

pub mod avatar;
pub mod deformer;
pub mod effects;
pub mod expression;
pub mod fixtures;
pub mod keyform;
pub mod mesh;
pub mod module;
pub mod motion;
pub mod parameter;
pub mod part;
pub mod physics;
pub mod pose;
pub mod standard;
pub mod unknown;
pub mod validate;

use serde::{Deserialize, Serialize};
use ugoe_core::Vec2;
/// `Vec2` のフィールドの生成用の型 (`#[ts(as = "crate::Vec2Repr")]` などで使う)。
#[cfg(any(feature = "ts", feature = "schema"))]
use ugoe_core::codegen::Vec2Repr;

pub use avatar::AvatarProfile;
pub use deformer::{Deformer, DeformerId, DeformerKind};
pub use effects::Effects;
pub use keyform::{
    DepthRotation, KeyformBinding, KeyformGrid, KeyformInference, KeyformInterpolation,
    KeyformOrigin, MirrorSource,
};
pub use mesh::{BlendMode, Glue, MaskMode, MaskRef, Mesh, MeshId};
pub use module::{DrawOrderRange, Module, ModulePhysics, ModuleRequirements, Slot, SlotId};
pub use parameter::{ParamId, Parameter, ParameterGroup};
pub use part::{Part, PartId};
pub use pose::Pose;

/// モデルデータのスキーマバージョン。互換性を壊す変更で上げる (`docs/spec/file-formats.md`)。
pub const SCHEMA_VERSION: u32 = 0;

/// 1 体のリグ付きモデル全体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Model {
    pub schema_version: u32,
    pub meta: ModelMeta,
    pub parameters: Vec<Parameter>,
    #[serde(default)]
    pub parameter_groups: Vec<ParameterGroup>,
    pub parts: Vec<Part>,
    #[serde(default)]
    pub deformers: Vec<Deformer>,
    pub meshes: Vec<Mesh>,
    #[serde(default)]
    pub glues: Vec<Glue>,
    pub textures: Vec<TextureRef>,
    #[serde(default)]
    pub hit_areas: Vec<HitArea>,
    /// 排他的なパーツ切り替え。省略時はグループなし (`docs/spec/model.md#ポーズ`)。
    #[serde(default)]
    pub pose: Pose,
    /// 目パチ・呼吸・リップシンクの設定。省略時は対象なし (`docs/spec/motion.md#エフェクト`)。
    #[serde(default)]
    pub effects: Effects,
    /// アバタープロファイル。省略時はなし (`docs/spec/avatar-profile.md`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<AvatarProfile>,
    /// 着せ替えのスロット。省略時はなし (`docs/spec/modules.md#スロット`)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelMeta {
    pub name: String,
    pub canvas: Canvas,
}

/// キャンバス。モデル座標系は「キャンバス左上原点・ピクセル単位・Y 下向き」。
/// 表示時の正規化 (高さ 2.0 の NDC 相当など) は描画側の責務。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Canvas {
    pub width: f32,
    pub height: f32,
    /// 表示時の原点 (通常は足元中央など)。
    #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
    #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
    pub origin: Vec2,
}

/// テクスチャ参照。実データはコンテナ (`ugoe-format`) 側が保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TextureRef {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

/// 当たり判定領域。メッシュの変形後形状で判定する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HitArea {
    pub id: String,
    pub mesh: MeshId,
}
