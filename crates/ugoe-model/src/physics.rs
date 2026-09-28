//! 物理演算 (振り子チェーン) の設定データ。アルゴリズムは `ugoe-physics`、仕様は `docs/spec/physics.md`。

use serde::{Deserialize, Serialize};
use ugoe_core::Vec2;

use crate::ParamId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicsSettings {
    pub gravity: Vec2,
    pub wind: Vec2,
    /// 固定ステップ実行の周波数 (Hz)。
    pub fps: f32,
    pub rigs: Vec<PhysicsRig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicsRig {
    pub id: String,
    pub name: String,
    pub inputs: Vec<PhysicsInput>,
    pub outputs: Vec<PhysicsOutput>,
    pub particles: Vec<PhysicsParticle>,
    pub normalization: PhysicsNormalization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicsSource {
    X,
    Y,
    Angle,
    /// 未知の値 (新しい版のファイル)。その入力・出力を無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicsInput {
    pub parameter: ParamId,
    pub source: PhysicsSource,
    /// 0..=100 (%)。
    pub weight: f32,
    #[serde(default)]
    pub reflect: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicsOutput {
    pub parameter: ParamId,
    /// 出力元となる粒子インデックス (その粒子と 1 つ前の粒子の成す角を使う)。
    pub particle: u32,
    pub source: PhysicsSource,
    pub scale: f32,
    pub weight: f32,
    #[serde(default)]
    pub reflect: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhysicsParticle {
    /// 初期位置 (根元からの相対)。
    pub position: Vec2,
    pub mobility: f32,
    pub delay: f32,
    pub acceleration: f32,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhysicsNormalization {
    pub position: NormalizationRange,
    pub angle: NormalizationRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NormalizationRange {
    pub min: f32,
    pub default: f32,
    pub max: f32,
}
