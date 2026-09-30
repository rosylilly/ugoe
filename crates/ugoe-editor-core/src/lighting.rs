//! エディタでのライティングの操作 (`docs/editor.md#ライティングの画面`)。
//!
//! - 法線マップを付けるコマンドを組み立てる: アルファの形からの生成 ([`generate_normal_map`]。AI は使わない。
//!   `ugoe_rigging::bulge_normal_map`) と、画像の割り当て ([`assign_normal_map`])。どちらも
//!   [`Command::SetNormalMap`] (モデルのライティングが無効なら [`Command::SetLighting`] も) を 1 つの
//!   [`Command::Batch`] にするので、1 回の Undo で戻る。
//! - プレビューの光源 ([`PreviewLighting`])。光源はモデルに入らない (ホストが渡す。`docs/spec/rendering.md#ライティング`) ので、
//!   `project.json` の `editor.lighting` に表示の設定として残す。

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_model::Model;
use ugoe_rigging::BulgeOptions;

use crate::command::Command;
use crate::project::ProjectMeta;
use crate::texture::{
    MAX_TEXTURE_NAME_LEN, Textures, decode_rgba_png, encode_rgba_png, is_valid_texture_name,
};

/// `project.json` の `editor` の中で、プレビューの光源を置くキー。
const LIGHTING_KEY: &str = "lighting";
/// 法線マップの画像の名前に付ける接尾辞 (`ugoe_ai::normal` と同じ)。
const SUFFIX: &str = "_normal";
/// プレビューの光源の数の上限 (参照レンダラが使う数、`ugoe_render::MAX_LIGHTS`)。
pub const MAX_PREVIEW_LIGHTS: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum LightingError {
    #[error("texture `{0}` not found")]
    TextureNotFound(String),
    #[error("the image of texture `{0}` is missing or is not a PNG")]
    UnreadableTexture(String),
    #[error("the normal map is not a readable PNG")]
    UnreadableNormalMap,
    #[error("normal map generation failed: {0}")]
    Generate(#[from] ugoe_rigging::NormalMapError),
    #[error("invalid preview lighting: {0}")]
    InvalidPreview(String),
    #[error("preview lighting: {0}")]
    Settings(#[source] serde_json::Error),
}

/// アルファの形からの法線マップの生成の設定 (`ugoe_rigging::BulgeOptions` のうち画面で選ぶもの)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct BulgeSettings {
    /// ふくらみの強さ (高さと幅の比)。1 で断面が円、0 で平ら。
    pub strength: f32,
    /// 縁の丸みの幅 (テクスチャの画素)。縁だけが丸まり中は平らになる。`None` なら形全体を 1 つのふくらみにする。
    pub edge: Option<f32>,
    /// 高さをならすぼかしの標準偏差 (画素)。
    pub smoothing: f32,
}

impl Default for BulgeSettings {
    fn default() -> Self {
        let d = BulgeOptions::default();
        Self {
            // 既定の 1 (断面が円) は顔や体のような大きなパーツには強い (T0506 のメモ)。
            strength: 0.5,
            edge: d.radius,
            smoothing: d.smoothing,
        }
    }
}

impl From<BulgeSettings> for BulgeOptions {
    fn from(s: BulgeSettings) -> Self {
        Self {
            strength: s.strength,
            radius: s.edge,
            smoothing: s.smoothing,
            ..Self::default()
        }
    }
}

/// テクスチャ `texture` のアルファの形からふくらみの法線マップを作り、付けるコマンドを返す。
pub fn generate_normal_map(
    model: &Model,
    textures: &Textures,
    texture: &str,
    settings: BulgeSettings,
) -> Result<Command, LightingError> {
    texture_index(model, texture)?;
    let png = textures
        .get(texture)
        .ok_or_else(|| LightingError::UnreadableTexture(texture.to_owned()))?;
    let (width, height, rgba) =
        decode_rgba_png(png).ok_or_else(|| LightingError::UnreadableTexture(texture.to_owned()))?;
    let map = ugoe_rigging::bulge_normal_map(width, height, &rgba, &settings.into())?;
    let png = encode_rgba_png(map.width, map.height, &map.rgba);
    set_normal_map_batch(model, texture, png, "Generate normal map")
}

/// PNG の画像を、テクスチャ `texture` の法線マップとして付けるコマンドを返す。
/// `flip_green` なら G を反転する (G が下向きの DirectX 形式の画像を、G が上向きの規約に直す)。
pub fn assign_normal_map(
    model: &Model,
    texture: &str,
    png: Vec<u8>,
    flip_green: bool,
) -> Result<Command, LightingError> {
    texture_index(model, texture)?;
    let (width, height, mut rgba) =
        decode_rgba_png(&png).ok_or(LightingError::UnreadableNormalMap)?;
    let png = if flip_green {
        for px in rgba.as_chunks_mut::<4>().0 {
            px[1] = 255 - px[1];
        }
        encode_rgba_png(width, height, &rgba)
    } else {
        png
    };
    set_normal_map_batch(model, texture, png, "Assign normal map")
}

fn texture_index(model: &Model, texture: &str) -> Result<usize, LightingError> {
    model
        .textures
        .iter()
        .position(|t| t.name == texture)
        .ok_or_else(|| LightingError::TextureNotFound(texture.to_owned()))
}

/// `SetNormalMap` と、モデルのライティングが無効なら `SetLighting { enabled: true }` を 1 つの Batch にする。
fn set_normal_map_batch(
    model: &Model,
    texture: &str,
    png: Vec<u8>,
    label: &str,
) -> Result<Command, LightingError> {
    let mut commands = vec![Command::SetNormalMap {
        texture: texture.to_owned(),
        normal_map: Some(normal_map_name(model, texture)),
        png: Some(png),
    }];
    if !model.lighting {
        commands.push(Command::SetLighting { enabled: true });
    }
    Ok(Command::Batch {
        label: label.to_owned(),
        commands,
    })
}

/// テクスチャ `texture` の法線マップの画像の名前。いまの法線マップをこのテクスチャだけが指していれば
/// その名前 (画像を差し替える)。ほかのテクスチャと共有していれば、それらを変えないよう新しい名前
/// (`<texture>_normal`、重なれば `_normal2`…) にする。
pub fn normal_map_name(model: &Model, texture: &str) -> String {
    let current = model
        .textures
        .iter()
        .find(|t| t.name == texture)
        .and_then(|t| t.normal_map.clone());
    if let Some(name) = current {
        let users = model
            .textures
            .iter()
            .filter(|t| t.normal_map.as_ref() == Some(&name))
            .count();
        if users == 1 {
            return name;
        }
    }
    let taken: Vec<String> = model
        .textures
        .iter()
        .flat_map(|t| std::iter::once(&t.name).chain(t.normal_map.as_ref()))
        .map(|n| n.to_ascii_lowercase())
        .collect();
    // テクスチャの名前は ASCII なので、バイトで切っても文字の途中にならない。
    let mut n = 1;
    loop {
        let suffix = if n == 1 {
            SUFFIX.to_owned()
        } else {
            format!("{SUFFIX}{n}")
        };
        let base = &texture[..texture.len().min(MAX_TEXTURE_NAME_LEN - suffix.len())];
        let name = format!("{base}{suffix}");
        if is_valid_texture_name(&name) && !taken.contains(&name.to_ascii_lowercase()) {
            return name;
        }
        n += 1;
    }
}

/// プレビューの光源 (`project.json` の `editor.lighting`)。エディタのビューポートとプレビューで
/// `WebPuppet::set_lighting_json` に渡す。モデルには入らない。すべて既定値なら書かない。
///
/// 座標はキャンバス座標 (ピクセル・Y 下向き) + 奥行き (手前が +z)。色は 0〜1 で、強さ `intensity` を掛けて渡す。
/// `project.json` に書く値が丸めの誤差で読みにくくならないよう、`f64` で持つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct PreviewLighting {
    /// ビューポートに光源を当てるか。`false` ならホストは光源を渡さない (環境光が白で光源なし)。
    pub enabled: bool,
    /// 環境光の色と強さ。
    pub ambient: [f64; 3],
    pub ambient_intensity: f64,
    /// 光源 (最大 [`MAX_PREVIEW_LIGHTS`] 個)。
    pub lights: Vec<PreviewLight>,
}

impl Default for PreviewLighting {
    /// 環境光を弱め、左上の手前から白い平行光源を 1 つ当てる。
    fn default() -> Self {
        Self {
            enabled: true,
            ambient: [1.0; 3],
            ambient_intensity: 0.45,
            lights: vec![PreviewLight::Directional {
                direction: [1.0, 1.0, -1.0],
                color: [1.0; 3],
                intensity: 0.7,
            }],
        }
    }
}

/// プレビューの光源 1 つ (`docs/spec/rendering.md#ライティング`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PreviewLight {
    /// 平行光源。`direction` は光が進む向き (長さは問わない。0 は不可)。
    Directional {
        direction: [f64; 3],
        color: [f64; 3],
        intensity: f64,
    },
    /// 点光源。`range` は光が届く距離 (`None` なら減衰しない)。
    Point {
        position: [f64; 3],
        color: [f64; 3],
        intensity: f64,
        #[serde(default)]
        range: Option<f64>,
    },
}

impl PreviewLighting {
    /// 値の範囲を検査する (有限・色と強さが負でない・向きが 0 でない・`range` が正・光源の数)。
    pub fn validate(&self) -> Result<(), LightingError> {
        let bad = |s: String| Err(LightingError::InvalidPreview(s));
        let color_ok =
            |c: &[f64; 3], i: f64| c.iter().chain([&i]).all(|v| v.is_finite() && *v >= 0.0);
        if !color_ok(&self.ambient, self.ambient_intensity) {
            return bad("ambient must be finite and not negative".into());
        }
        if self.lights.len() > MAX_PREVIEW_LIGHTS {
            return bad(format!("at most {MAX_PREVIEW_LIGHTS} lights"));
        }
        for (i, light) in self.lights.iter().enumerate() {
            match light {
                PreviewLight::Directional {
                    direction,
                    color,
                    intensity,
                } => {
                    if !color_ok(color, *intensity) {
                        return bad(format!(
                            "lights[{i}]: color must be finite and not negative"
                        ));
                    }
                    if !direction.iter().all(|v| v.is_finite()) || *direction == [0.0; 3] {
                        return bad(format!(
                            "lights[{i}]: direction must be finite and not zero"
                        ));
                    }
                }
                PreviewLight::Point {
                    position,
                    color,
                    intensity,
                    range,
                } => {
                    if !color_ok(color, *intensity) {
                        return bad(format!(
                            "lights[{i}]: color must be finite and not negative"
                        ));
                    }
                    if !position.iter().all(|v| v.is_finite()) {
                        return bad(format!("lights[{i}]: position must be finite"));
                    }
                    if range.is_some_and(|r| !(r.is_finite() && r > 0.0)) {
                        return bad(format!("lights[{i}]: range must be positive"));
                    }
                }
            }
        }
        Ok(())
    }
}

impl ProjectMeta {
    /// プロジェクトに残したプレビューの光源。ない・読めなければ既定値。
    pub fn preview_lighting(&self) -> PreviewLighting {
        self.editor
            .get(LIGHTING_KEY)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .filter(|l: &PreviewLighting| l.validate().is_ok())
            .unwrap_or_default()
    }

    /// プレビューの光源を残す。既定値なら `editor.lighting` を消す (既存のプロジェクトの `project.json` を変えないため)。
    /// 設定が変わったら `true`。
    pub fn set_preview_lighting(
        &mut self,
        lighting: PreviewLighting,
    ) -> Result<bool, LightingError> {
        lighting.validate()?;
        let changed = self.preview_lighting() != lighting;
        if lighting == PreviewLighting::default() {
            self.editor.remove(LIGHTING_KEY);
        } else {
            let value = serde_json::to_value(lighting).map_err(LightingError::Settings)?;
            self.editor.insert(LIGHTING_KEY.to_owned(), value);
        }
        Ok(changed)
    }
}
