use serde::{Deserialize, Serialize};
use ugoe_core::{Id, Rgb, Vec2};

use crate::{DeformerId, KeyformGrid, PartId};

pub type MeshId = Id<Mesh>;

/// 描画単位となるテクスチャ付き三角形メッシュ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mesh {
    pub id: MeshId,
    pub name: String,
    pub part: PartId,
    /// `None` ならキャンバス座標直下。
    pub parent: Option<DeformerId>,
    /// `Model::textures` のインデックス。
    pub texture: u32,
    /// テクスチャ座標 (0..1, 左上原点)。頂点数と同じ長さ。
    pub uvs: Vec<Vec2>,
    /// 三角形インデックス (3 の倍数)。
    pub indices: Vec<u16>,
    #[serde(default)]
    pub blend: BlendMode,
    /// 裏面カリングするか。
    #[serde(default)]
    pub culling: bool,
    /// クリッピングマスクの参照。意味論は `docs/spec/rendering.md#マスク`。
    #[serde(default)]
    pub masks: Vec<MaskRef>,
    /// 通常マスク (`invert = false` の参照) の結合方法。
    #[serde(default)]
    pub mask_mode: MaskMode,
    /// `true` なら色としては描画せず、マスクとしてだけ働く。
    #[serde(default)]
    pub mask_only: bool,
    pub keyforms: KeyformGrid<MeshKeyform>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshKeyform {
    /// 親座標系での頂点位置。長さは `uvs` と同じ。
    pub positions: Vec<Vec2>,
    pub opacity: f32,
    /// 描画順 (大きいほど手前)。同値はモデル内の宣言順で解決。
    pub draw_order: f32,
    #[serde(default = "white")]
    pub multiply_color: Rgb,
    #[serde(default = "black")]
    pub screen_color: Rgb,
    /// マスクの効き具合 (0〜1、既定 1)。範囲外の値はランタイムがクランプする。
    #[serde(default = "one")]
    pub mask_strength: f32,
}

/// マスクとして使うメッシュへの参照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaskRef {
    pub mesh: MeshId,
    /// `true` なら反転マスク (メッシュの内側を隠す)。
    #[serde(default)]
    pub invert: bool,
}

impl MaskRef {
    /// 通常マスクの参照。
    pub fn normal(mesh: impl Into<MeshId>) -> Self {
        Self {
            mesh: mesh.into(),
            invert: false,
        }
    }

    /// 反転マスクの参照。
    pub fn inverted(mesh: impl Into<MeshId>) -> Self {
        Self {
            mesh: mesh.into(),
            invert: true,
        }
    }
}

/// 通常マスクの結合方法。式は `docs/spec/rendering.md#マスク`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskMode {
    /// 和集合 `1 − Π(1 − c)`。
    #[default]
    Union,
    /// 共通部分 `Π c`。
    Intersection,
}

fn one() -> f32 {
    1.0
}

fn white() -> Rgb {
    Rgb::WHITE
}

fn black() -> Rgb {
    Rgb::BLACK
}

/// 合成モード。式は `docs/spec/rendering.md`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    #[default]
    Normal,
    Additive,
    Multiply,
}

/// 2 つのメッシュの頂点同士を引き寄せて継ぎ目を防ぐ「接着」。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Glue {
    pub id: String,
    pub mesh_a: MeshId,
    pub mesh_b: MeshId,
    pub pairs: Vec<GluePair>,
    pub keyforms: KeyformGrid<GlueKeyform>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GluePair {
    pub a: u16,
    pub b: u16,
    /// 0.0 なら a 側に、1.0 なら b 側に寄せる。
    pub weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GlueKeyform {
    pub intensity: f32,
}
