//! 光源 (`docs/spec/rendering.md#ライティング`)。
//!
//! 光源はモデルのデータではなく、ホストがフレームごとにレンダラへ渡す
//! (参照レンダラなら `ugoe_render::Renderer::set_lighting`)。
//! GPU に依存しない組み込み先 (C ABI・GDExtension) も同じ型を使えるように、ここに置く (T0166)。
//! `ugoe` は `ugoe::lighting`、`ugoe-render` は `ugoe_render::{Light, Lighting, MAX_LIGHTS}` として再エクスポートする。

use alloc::vec::Vec;

use crate::Rgb;

/// 参照レンダラが 1 フレームに使う光源の数の上限。使える光源のうち、これより後ろのものは使わない。
pub const MAX_LIGHTS: usize = 8;

/// ホストが渡す光源の組 (`docs/spec/rendering.md#ライティング`)。
///
/// 既定値は環境光が白 (1, 1, 1) で光源なし。ライティングを受ける要素も、受けない場合と同じ色で描かれる。
#[derive(Debug, Clone, PartialEq)]
pub struct Lighting {
    /// 環境光。法線によらず掛かる。
    pub ambient: Rgb,
    /// 光源。使える光源 ([`Light`]) のうち先頭の [`MAX_LIGHTS`] 個だけを使う。
    pub lights: Vec<Light>,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            ambient: Rgb::WHITE,
            lights: Vec::new(),
        }
    }
}

/// 光源 1 つ。位置と向きはキャンバス座標 (ピクセル単位・Y 下向き) + 奥行き (z、キャンバスから手前向き)。
///
/// 色は 1 を超えてもよい (強さを兼ねる)。
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Light {
    /// 平行光源。`direction` は光が進む向き (長さは問わない)。
    /// 例: 左上の手前から照らすなら `[1.0, 1.0, -1.0]`。長さが 0 か有限でなければ使わない。
    Directional {
        /// 光が進む向き (x, y, z)。
        direction: [f32; 3],
        /// 光の色 (強さを兼ねる)。
        color: Rgb,
    },
    /// 点光源。`position` の位置から全方向を照らし、距離 `range` で 0 になるように減衰する
    /// (`(1 − d / range)²`)。`range` が `f32::INFINITY` なら減衰しない。
    /// `range` が正でない (NaN を含む) か、位置が有限でなければ使わない。
    Point {
        /// 光源の位置 (x, y, z)。
        position: [f32; 3],
        /// 光の色 (強さを兼ねる)。
        color: Rgb,
        /// 光が届く距離 (キャンバスのピクセル単位)。
        range: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_white_ambient_without_lights() {
        let lighting = Lighting::default();
        assert_eq!(lighting.ambient, Rgb::WHITE);
        assert!(lighting.lights.is_empty());
    }
}
