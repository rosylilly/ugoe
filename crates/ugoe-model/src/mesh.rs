use serde::{Deserialize, Serialize};
use ugoe_core::{Id, Rgb, Vec2};

use crate::{DeformerId, KeyformGrid, PartId};

/// メッシュの ID。モデルの中で一意。
pub type MeshId = Id<Mesh>;
/// グルーの ID。モデルの中で一意。
pub type GlueId = Id<Glue>;

/// 描画単位となるテクスチャ付き三角形メッシュ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Mesh {
    /// モデルの中で一意な ID。
    pub id: MeshId,
    /// 表示名 (エディタ用)。
    pub name: String,
    /// 所属するパーツ。パーツの不透明度が掛かる。
    pub part: PartId,
    /// `None` ならキャンバス座標直下。
    pub parent: Option<DeformerId>,
    /// `Model::textures` のインデックス。
    pub texture: u32,
    /// テクスチャ座標 (0..1, 左上原点)。頂点数と同じ長さ。
    #[cfg_attr(feature = "ts", ts(as = "Vec<crate::Vec2Repr>"))]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<crate::Vec2Repr>"))]
    pub uvs: Vec<Vec2>,
    /// 三角形インデックス (3 の倍数)。
    pub indices: Vec<u16>,
    /// 合成モード。
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
    /// 頂点位置・不透明度・描画順・色のキーフォーム。
    pub keyforms: KeyformGrid<MeshKeyform>,
    /// ミラーで対にするメッシュ (オーサリング用)。`None` と自分自身は「自分と対応する」。
    /// 対は双方向に張る (`docs/spec/keyform-inference.md#ミラー`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<MeshId>,
    /// ミラーの対称軸 (縦の直線) の x 座標。頂点と同じ親座標系で表す (オーサリング用)。
    /// `None` なら親で決まる既定値 (`docs/spec/keyform-inference.md#対称軸`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_axis: Option<f32>,
    /// 頂点ごとの深度 (キャンバスのピクセル単位、手前が正。オーサリング用)。長さは `uvs` と同じ。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<Vec<f32>>,
    /// ライティングを受けるか (既定 `true`、`true` は書き出しでも省略する)。
    /// `Model::lighting` が `true` のときだけ効く (`docs/spec/rendering.md#ライティング`)。
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub lit: bool,
}

/// メッシュの 1 つのキーフォーム。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MeshKeyform {
    /// 親座標系での頂点位置。長さは `uvs` と同じ。
    #[cfg_attr(feature = "ts", ts(as = "Vec<crate::Vec2Repr>"))]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<crate::Vec2Repr>"))]
    pub positions: Vec<Vec2>,
    /// 不透明度 (0〜1)。
    pub opacity: f32,
    /// 描画順 (大きいほど手前)。同値はモデル内の宣言順で解決。
    pub draw_order: f32,
    /// 乗算色 (既定は白 = 変化なし)。式は `docs/spec/rendering.md`。
    #[serde(default = "white")]
    pub multiply_color: Rgb,
    /// スクリーン色 (既定は黒 = 変化なし)。式は `docs/spec/rendering.md`。
    #[serde(default = "black")]
    pub screen_color: Rgb,
    /// マスクの効き具合 (0〜1、既定 1)。範囲外の値はランタイムがクランプする。
    #[serde(default = "one")]
    pub mask_strength: f32,
}

/// マスクとして使うメッシュへの参照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MaskRef {
    /// マスクにするメッシュ。
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum MaskMode {
    /// 和集合 `1 − Π(1 − c)`。
    #[default]
    Union,
    /// 共通部分 `Π c`。
    Intersection,
    /// 未知の値 (新しい版のファイル)。和集合として扱う。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl MaskMode {
    /// フォールバックを適用した結合方法 (`Unknown` は和集合)。
    pub fn effective(self) -> Self {
        match self {
            Self::Unknown => Self::Union,
            mode => mode,
        }
    }
}

fn yes() -> bool {
    true
}

fn is_yes(value: &bool) -> bool {
    *value
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    /// 通常の合成 (`src + dst * (1 - src.a)`)。
    #[default]
    Normal,
    /// 加算合成。
    Additive,
    /// 乗算合成。
    Multiply,
    /// 未知の値 (新しい版のファイル)。通常の合成で描く。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl BlendMode {
    /// フォールバックを適用した合成モード (`Unknown` は通常の合成)。
    pub fn effective(self) -> Self {
        match self {
            Self::Unknown => Self::Normal,
            mode => mode,
        }
    }
}

/// 2 つのメッシュの頂点同士を引き寄せて継ぎ目を防ぐ「接着」。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Glue {
    /// モデルの中で一意な ID。
    pub id: GlueId,
    /// 接着する一方のメッシュ (`GluePair::a` の頂点を持つ)。
    pub mesh_a: MeshId,
    /// 接着するもう一方のメッシュ (`GluePair::b` の頂点を持つ)。
    pub mesh_b: MeshId,
    /// 引き寄せる頂点のペア。この順に適用する。
    pub pairs: Vec<GluePair>,
    /// 引き寄せの強さのキーフォーム。
    pub keyforms: KeyformGrid<GlueKeyform>,
}

/// グルーで引き寄せる頂点のペア。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GluePair {
    /// `Glue::mesh_a` の頂点のインデックス。
    pub a: u16,
    /// `Glue::mesh_b` の頂点のインデックス。
    pub b: u16,
    /// 0.0 なら a 側に、1.0 なら b 側に寄せる。
    pub weight: f32,
}

/// グルーの 1 つのキーフォーム。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GlueKeyform {
    /// 引き寄せの強さ (0 なら引き寄せない、1 なら `weight` の位置に一致させる)。補間後の値はクランプしない。
    pub intensity: f32,
}
