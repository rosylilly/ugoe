//! 評価の結果 1 フレーム分 (`<ケース>.json`)。
//!
//! 形は公開仕様の評価の例の `expected.json` のケース (`spec/examples/README.md#files`) と同じで、
//! `meshes` (メッシュの ID → 出力) と `render_order` (奥から手前へのメッシュの ID) を持つ。
//! バインディングが取り出せない値は省いてよい (比べるのは両方にある値だけ)。`positions` と `render_order` は必須。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ugoe::runtime::ModelInstance;

/// 1 フレーム分の評価の結果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub meshes: BTreeMap<String, MeshFrame>,
    pub render_order: Vec<String>,
    /// 時間を進める間に発火したモーションのイベント (計画の `motions` のあるケースだけ。T0462)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion_events: Option<Vec<MotionEventFrame>>,
}

/// 発火したモーションのイベント 1 つと、それを発火したモーション (`ugoe::Puppet::motion_event_sources`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionEventFrame {
    /// 発火した `update` の番号 (計画の `steps` の番号)。
    pub step: usize,
    pub value: String,
    /// モーションの中の時刻 (秒)。
    pub time: f32,
    /// 発火したモーションの名前。
    pub motion: String,
    /// 発火したモーションを始めた番号 (1 から)。
    pub number: u64,
}

/// メッシュ 1 つの出力 (`docs/spec/evaluation.md#8-出力`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshFrame {
    /// 頂点位置 (キャンバス座標)。
    pub positions: Vec<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draw_order: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiply_color: Option<Rgb>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen_color: Option<Rgb>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask_strength: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
}

/// 色 (`{"r", "g", "b"}`。公開仕様の例と同じ形)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl From<ugoe::core::Rgb> for Rgb {
    fn from(c: ugoe::core::Rgb) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
        }
    }
}

impl Frame {
    /// 評価済みのインスタンスから作る (すべての値を持つ)。
    pub fn from_instance(instance: &ModelInstance) -> Self {
        let model = instance.compiled().model();
        let id = |i: usize| model.meshes[i].id.to_string();
        let meshes = instance
            .drawables()
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let mesh = MeshFrame {
                    positions: d.positions.iter().map(|p| [p.x, p.y]).collect(),
                    opacity: Some(d.opacity),
                    draw_order: Some(d.draw_order),
                    multiply_color: Some(d.multiply_color.into()),
                    screen_color: Some(d.screen_color.into()),
                    mask_strength: Some(d.mask_strength),
                    visible: Some(d.visible),
                };
                (id(i), mesh)
            })
            .collect();
        let render_order = instance
            .render_order()
            .iter()
            .map(|&i| id(i as usize))
            .collect();
        Self {
            meshes,
            render_order,
            motion_events: None,
        }
    }
}
