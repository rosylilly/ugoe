//! ugoe の **データ定義 (スキーマ) クレート**。
//!
//! ここにはシリアライズ可能な「モデルが何であるか」の定義だけを置き、
//! 「どう動かすか」(補間・変形・物理・モーション再生) は下流クレートが担う。
//! 仕様は `docs/spec/model.md` を正とする。型を変更したら仕様書も同じ PR で更新すること。
//!
//! `no_std` + `alloc` で書く。既定で有効な feature `std` を外しても公開 API は変わらない
//! (`docs/portability.md#ランタイム層が守る制約-移植性のため`)。
//! ただしコード生成の feature (`ts`・`schema`) を有効にしたときだけは std の prelude を使う
//! (ts-rs と schemars の derive が生成するコードが `String`・`format!` などを prelude から使うため。ツール層だけが有効にする)。

#![cfg_attr(not(any(feature = "ts", feature = "schema")), no_std)]
#![warn(missing_docs)]

extern crate alloc;

pub mod avatar;
/// デフォーマ (ワープ・回転)。
pub mod deformer;
pub mod effects;
/// 表情。
pub mod expression;
pub mod fixtures;
pub mod keyform;
/// メッシュ・マスク・合成モード・グルー。
pub mod mesh;
pub mod module;
pub mod motion;
/// パラメータとパラメータグループ。
pub mod parameter;
/// パーツ。
pub mod part;
pub mod physics;
pub mod pose;
#[cfg(feature = "schema")]
pub mod spec_files;
pub mod standard;
pub mod unknown;
pub mod validate;

use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};
/// `Vec2` のフィールドの生成用の型 (`#[ts(as = "crate::Vec2Repr")]` などで使う)。
#[cfg(any(feature = "ts", feature = "schema"))]
use ugoe_core::codegen::Vec2Repr;
use ugoe_core::{Id, Vec2};

pub use avatar::AvatarProfile;
pub use deformer::{Deformer, DeformerId, DeformerKind};
pub use effects::Effects;
pub use keyform::{
    DepthRotation, KeyformBinding, KeyformGrid, KeyformInference, KeyformInterpolation,
    KeyformOrigin, MirrorSource,
};
pub use mesh::{BlendMode, Glue, GlueId, MaskMode, MaskRef, Mesh, MeshId};
pub use module::{
    DrawOrderRange, Module, ModulePhysics, ModulePose, ModuleRequirements, Slot, SlotId,
};
pub use parameter::{ParamId, Parameter, ParameterGroup, ParameterGroupId};
pub use part::{Part, PartId};
pub use pose::Pose;

/// モデルデータのスキーマバージョン。互換性を壊す変更で上げる (`docs/spec/file-formats.md`)。
pub const SCHEMA_VERSION: u32 = 0;

/// 1 体のリグ付きモデル全体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Model {
    /// このデータのスキーマバージョン ([`SCHEMA_VERSION`])。
    pub schema_version: u32,
    /// 名前とキャンバス。
    pub meta: ModelMeta,
    /// パラメータ。キーフォーム・モーション・物理などが ID で参照する。
    pub parameters: Vec<Parameter>,
    /// エディタ表示用のパラメータグループ。
    #[serde(default)]
    pub parameter_groups: Vec<ParameterGroup>,
    /// パーツ。階層は各パーツの `parent` で表す。
    pub parts: Vec<Part>,
    /// デフォーマ。評価は親が先の順で、結果は配列の順序に依存しない (`docs/spec/evaluation.md`)。
    #[serde(default)]
    pub deformers: Vec<Deformer>,
    /// メッシュ。
    pub meshes: Vec<Mesh>,
    /// メッシュの頂点同士の接着。配列の順に適用する (`docs/spec/evaluation.md#5-グルー`)。
    #[serde(default)]
    pub glues: Vec<Glue>,
    /// テクスチャの参照。メッシュの `texture` はこの列のインデックス。
    pub textures: Vec<TextureRef>,
    /// 当たり判定。ホストがクリック・タップされた部位を調べるのに使う。
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
    /// ホストが渡す光源でライティングするか。省略時は `false` で、`false` は書き出しでも省略する。
    /// メッシュごとの有無は `Mesh::lit` (`docs/spec/rendering.md#ライティング`)。
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub lighting: bool,
}

/// モデルのメタデータ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelMeta {
    /// モデルの名前。
    pub name: String,
    /// キャンバス (モデル座標系の範囲)。
    pub canvas: Canvas,
}

/// キャンバス。モデル座標系は「キャンバス左上原点・ピクセル単位・Y 下向き」。
/// 表示時の正規化 (高さ 2.0 の NDC 相当など) は描画側の責務。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Canvas {
    /// キャンバスの幅 (ピクセル)。
    pub width: f32,
    /// キャンバスの高さ (ピクセル)。
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
    /// テクスチャの画像の名前。コンテナの中の画像を指す。
    pub name: String,
    /// 画像の幅 (ピクセル)。
    pub width: u32,
    /// 画像の高さ (ピクセル)。
    pub height: u32,
    /// 法線マップの画像の名前。`None` なら法線マップなし (法線は `(0, 0, 1)`)。
    /// 画像は同じ UV 空間の接空間法線で、バンドルでは `NRML` チャンクに入る
    /// (`docs/spec/rendering.md#ライティング`、`docs/spec/file-formats.md`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal_map: Option<String>,
}

/// 当たり判定の ID。モデルの中で一意。
pub type HitAreaId = Id<HitArea>;

/// 当たり判定領域。メッシュの変形後形状で判定する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HitArea {
    /// モデルの中で一意な ID。ホストはこの ID で当たり判定を参照する。
    pub id: HitAreaId,
    /// 判定に使うメッシュ。
    pub mesh: MeshId,
}
