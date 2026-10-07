use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};
use ugoe_core::{Id, Vec2};

use crate::{KeyformGrid, PartId};

/// デフォーマの ID。モデルの中で一意。
pub type DeformerId = Id<Deformer>;

/// デフォーマ: 子 (デフォーマ / メッシュ) の頂点を変形する。
/// 子の頂点座標は **親デフォーマの局所座標系** で表現される (`docs/spec/evaluation.md`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Deformer {
    /// モデルの中で一意な ID。
    pub id: DeformerId,
    /// 表示名 (エディタ用)。
    pub name: String,
    /// 所属するパーツ。パーツの不透明度が子に掛かる。
    pub part: PartId,
    /// `None` ならキャンバス座標直下。
    pub parent: Option<DeformerId>,
    /// 変形の種類と、種類ごとのキーフォーム。
    pub kind: DeformerKind,
    /// ミラーで対にするデフォーマ (オーサリング用)。`None` と自分自身は「自分と対応する」。
    /// 対は双方向に張る (`docs/spec/keyform-inference.md#ミラー`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<DeformerId>,
    /// ミラーの対称軸 (縦の直線) の x 座標。原点・制御点と同じ親座標系で表す (オーサリング用)。
    /// `None` なら親で決まる既定値 (`docs/spec/keyform-inference.md#対称軸`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_axis: Option<f32>,
}

/// デフォーマの種類。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeformerKind {
    /// 格子による自由変形。
    Warp(WarpDeformer),
    /// 原点まわりの回転・拡縮・反転。
    Rotation(RotationDeformer),
    /// 未知の値 (新しい版のファイル)。恒等変換として扱う (子は親の変換だけを受ける)。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

/// ワープデフォーマ: (rows+1)×(cols+1) 個の制御点を持つ格子による自由変形。
/// 子の局所座標は格子全体を `[0,1]²` に正規化した UV 空間。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WarpDeformer {
    /// 格子の行数 (縦の分割数)。制御点は縦に `rows + 1` 個。
    pub rows: u16,
    /// 格子の列数 (横の分割数)。制御点は横に `cols + 1` 個。
    pub cols: u16,
    /// 制御点と不透明度のキーフォーム。
    pub keyforms: KeyformGrid<WarpKeyform>,
    /// 制御点ごとの深度 (キャンバスのピクセル単位、手前が正。オーサリング用)。
    /// 並びは `WarpKeyform::points` と同じで、長さ `(rows+1)*(cols+1)`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<Vec<f32>>,
}

/// ワープデフォーマの 1 つのキーフォーム。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WarpKeyform {
    /// 行優先 (x が速い) の制御点。長さ `(rows+1)*(cols+1)`。親座標系。
    #[cfg_attr(feature = "ts", ts(as = "Vec<crate::Vec2Repr>"))]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<crate::Vec2Repr>"))]
    pub points: Vec<Vec2>,
    /// 子に掛ける不透明度 (0〜1)。
    pub opacity: f32,
}

/// 回転デフォーマ: 原点まわりの回転・拡縮・反転。
/// 子の局所座標は原点を (0,0) とした回転前の座標 (単位はキャンバスピクセル)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RotationDeformer {
    /// 原点・角度・拡大率・反転・不透明度のキーフォーム。
    pub keyforms: KeyformGrid<RotationKeyform>,
}

/// 回転デフォーマの 1 つのキーフォーム。変換の式は `docs/spec/evaluation.md`。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RotationKeyform {
    /// 親座標系での原点。
    #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
    #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
    pub origin: Vec2,
    /// 度数法、時計回り正 (Y 下向き座標系)。
    pub angle: f32,
    /// 拡大率 (1 で等倍)。
    pub scale: f32,
    /// `true` なら局所座標の x を反転する。
    #[serde(default)]
    pub reflect_x: bool,
    /// `true` なら局所座標の y を反転する。
    #[serde(default)]
    pub reflect_y: bool,
    /// 子に掛ける不透明度 (0〜1)。
    pub opacity: f32,
}
