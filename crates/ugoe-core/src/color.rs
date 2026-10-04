use serde::{Deserialize, Serialize};

/// リニアではなく sRGB 値の RGB (0.0..=1.0)。乗算色・スクリーン色に使う。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Rgb {
    /// 赤 (0.0..=1.0)。
    pub r: f32,
    /// 緑 (0.0..=1.0)。
    pub g: f32,
    /// 青 (0.0..=1.0)。
    pub b: f32,
}

impl Rgb {
    /// 白。乗算色の既定値 (色を変えない)。
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };
    /// 黒。スクリーン色の既定値 (色を変えない)。
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };

    /// 各成分 (0.0..=1.0) から作る。範囲の外の値は切り詰めない。
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}
