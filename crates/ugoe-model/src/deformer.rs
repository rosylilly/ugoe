use serde::{Deserialize, Serialize};
use ugoe_core::{Id, Vec2};

use crate::{KeyformGrid, PartId};

pub type DeformerId = Id<Deformer>;

/// デフォーマ: 子 (デフォーマ / メッシュ) の頂点を変形する。
/// 子の頂点座標は **親デフォーマの局所座標系** で表現される (`docs/spec/evaluation.md`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deformer {
    pub id: DeformerId,
    pub name: String,
    pub part: PartId,
    /// `None` ならキャンバス座標直下。
    pub parent: Option<DeformerId>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeformerKind {
    Warp(WarpDeformer),
    Rotation(RotationDeformer),
    /// 未知の値 (新しい版のファイル)。恒等変換として扱う (子は親の変換だけを受ける)。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    Unknown,
}

/// ワープデフォーマ: (rows+1)×(cols+1) 個の制御点を持つ格子による自由変形。
/// 子の局所座標は格子全体を [0,1]² に正規化した UV 空間。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarpDeformer {
    pub rows: u16,
    pub cols: u16,
    pub keyforms: KeyformGrid<WarpKeyform>,
    /// 制御点ごとの深度 (キャンバスのピクセル単位、手前が正。オーサリング用)。
    /// 並びは `WarpKeyform::points` と同じで、長さ `(rows+1)*(cols+1)`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<Vec<f32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarpKeyform {
    /// 行優先 (x が速い) の制御点。長さ `(rows+1)*(cols+1)`。親座標系。
    pub points: Vec<Vec2>,
    pub opacity: f32,
}

/// 回転デフォーマ: 原点まわりの回転・拡縮・反転。
/// 子の局所座標は原点を (0,0) とした回転前の座標 (単位はキャンバスピクセル)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotationDeformer {
    pub keyforms: KeyformGrid<RotationKeyform>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RotationKeyform {
    /// 親座標系での原点。
    pub origin: Vec2,
    /// 度数法、時計回り正 (Y 下向き座標系)。
    pub angle: f32,
    pub scale: f32,
    #[serde(default)]
    pub reflect_x: bool,
    #[serde(default)]
    pub reflect_y: bool,
    pub opacity: f32,
}
