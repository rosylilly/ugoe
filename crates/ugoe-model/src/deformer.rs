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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeformerKind {
    Warp(WarpDeformer),
    Rotation(RotationDeformer),
}

/// ワープデフォーマ: (rows+1)×(cols+1) 個の制御点を持つ格子による自由変形。
/// 子の局所座標は格子全体を [0,1]² に正規化した UV 空間。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarpDeformer {
    pub rows: u16,
    pub cols: u16,
    pub keyforms: KeyformGrid<WarpKeyform>,
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
