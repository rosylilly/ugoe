//! アルファ画像からの自動メッシュ生成。仕様: `docs/ai-integration.md#自動メッシュ`。
//!
//! パイプライン: アルファ二値化 → 輪郭抽出 → 外側へのオフセット・簡略化 →
//! 内部点の配置 → 制約付きドロネー三角形分割。

use ugoe_core::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshOptions {
    /// この値以上のアルファを「中身」とみなす (0..=255)。
    pub alpha_threshold: u8,
    /// 輪郭を外側に広げる距離 (px)。
    pub margin: f32,
    /// 輪郭上の頂点間隔の目安 (px)。
    pub edge_spacing: f32,
    /// 内部頂点の間隔の目安 (px)。0 なら内部点を置かない。
    pub interior_spacing: f32,
}

impl Default for MeshOptions {
    fn default() -> Self {
        Self {
            alpha_threshold: 8,
            margin: 4.0,
            edge_spacing: 24.0,
            interior_spacing: 48.0,
        }
    }
}

/// 生成結果。座標は入力画像のピクセル座標。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GeneratedMesh {
    pub vertices: Vec<Vec2>,
    pub indices: Vec<u16>,
}

#[derive(Debug, thiserror::Error)]
pub enum MeshError {
    #[error("image is fully transparent")]
    Empty,
    #[error("rgba length {len} does not match {width}x{height}")]
    BadImage { width: u32, height: u32, len: usize },
}

/// 非乗算 RGBA8 画像からメッシュを生成する。
pub fn generate(
    rgba: &[u8],
    width: u32,
    height: u32,
    options: &MeshOptions,
) -> Result<GeneratedMesh, MeshError> {
    if rgba.len() != (width * height * 4) as usize {
        return Err(MeshError::BadImage {
            width,
            height,
            len: rgba.len(),
        });
    }
    // TODO(T0502): 輪郭抽出 + 三角形分割。暫定としてアルファ外接矩形の四角形を返す。
    let mut min = (u32::MAX, u32::MAX);
    let mut max = (0, 0);
    for y in 0..height {
        for x in 0..width {
            if rgba[((y * width + x) * 4 + 3) as usize] >= options.alpha_threshold {
                min = (min.0.min(x), min.1.min(y));
                max = (max.0.max(x + 1), max.1.max(y + 1));
            }
        }
    }
    if min.0 > max.0 {
        return Err(MeshError::Empty);
    }
    let (x0, y0) = (min.0 as f32 - options.margin, min.1 as f32 - options.margin);
    let (x1, y1) = (max.0 as f32 + options.margin, max.1 as f32 + options.margin);
    Ok(GeneratedMesh {
        vertices: vec![
            Vec2::new(x0, y0),
            Vec2::new(x1, y0),
            Vec2::new(x1, y1),
            Vec2::new(x0, y1),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
    })
}
