//! アルファ画像からの自動メッシュ生成。仕様: `docs/ai-integration.md#自動メッシュ`。
//!
//! パイプライン: アルファ二値化 → (小さな島の除去) → `margin` だけ膨張 → 輪郭抽出
//! (marching squares) → 簡略化 (Douglas–Peucker)・再サンプリング → 内部点の配置 (六角格子) →
//! 制約付きドロネー三角形分割 (`spade`) → 輪郭の内側の三角形だけを残す。
//! 頂点数が `u16` に収まらなければ間隔を広げてやり直す。詳細はクレートの README を参照。
//!
//! エディタでメッシュを作り直すための、点群の三角形分割 ([`triangulate_points`]) も提供する。

mod contour;
mod mask;
mod points;
mod triangulate;

use ugoe_core::Vec2;

use crate::contour::P;
use crate::mask::Mask;
pub use crate::points::{TriangulateError, triangulate_points};

/// 出力できる頂点数の上限。インデックス `0xFFFF` はプリミティブリスタートと衝突しうるので使わない。
pub const MAX_VERTICES: usize = u16::MAX as usize;

/// 間隔を自動で広げてやり直す回数の上限。
const MAX_ATTEMPTS: u32 = 24;
/// やり直すたびに間隔に掛ける倍率。
const SPACING_GROWTH: f64 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshOptions {
    /// この値以上のアルファを「中身」とみなす (0..=255)。
    pub alpha_threshold: u8,
    /// 輪郭を外側に広げる距離 (px)。簡略化で最大半分ほど内側に戻ることがある。
    pub margin: f32,
    /// 輪郭上の頂点間隔の目安 (px)。これより長い辺は等分する。
    pub edge_spacing: f32,
    /// 内部頂点の間隔の目安 (px)。0 なら内部点を置かない。
    pub interior_spacing: f32,
    /// 画素数 (8 近傍で連結) がこれ未満の島を捨てる。0 か 1 なら何も捨てない。
    /// 捨てた島の画素はメッシュに覆われない。
    pub min_island_area: u32,
}

impl Default for MeshOptions {
    fn default() -> Self {
        Self {
            alpha_threshold: 8,
            margin: 4.0,
            edge_spacing: 24.0,
            interior_spacing: 48.0,
            min_island_area: 0,
        }
    }
}

/// 生成結果。座標は入力画像のピクセル座標 (Y 下向き)。
///
/// 三角形は画面上で反時計回り (`docs/spec/rendering.md#裏面カリング` の「表」) に並ぶ。
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
    #[error("invalid mesh option: {0}")]
    InvalidOptions(&'static str),
    #[error("shape is too complex to fit in 65535 vertices")]
    TooComplex,
}

/// 非乗算 RGBA8 画像からメッシュを生成する。
///
/// 同じ入力からは常に同じメッシュを返す。`min_island_area` で捨てた島を除き、
/// アルファが `alpha_threshold` 以上のすべての画素の中心は、いずれかの三角形に含まれる。
pub fn generate(
    rgba: &[u8],
    width: u32,
    height: u32,
    options: &MeshOptions,
) -> Result<GeneratedMesh, MeshError> {
    let expected = u64::from(width) * u64::from(height) * 4;
    if rgba.len() as u64 != expected {
        return Err(MeshError::BadImage {
            width,
            height,
            len: rgba.len(),
        });
    }
    validate(options)?;
    let margin = f64::from(options.margin);
    let edge_spacing = f64::from(options.edge_spacing);
    let interior_spacing = f64::from(options.interior_spacing);

    // 1. 二値化。膨張した領域が収まるよう周囲に余白を付けた格子で扱う。
    let pad = margin.ceil() as usize + 2;
    let (w, h) = (width as usize, height as usize);
    let mut opaque = Mask::new(w + 2 * pad, h + 2 * pad);
    for y in 0..h {
        for x in 0..w {
            opaque.cells[(y + pad) * opaque.width + x + pad] =
                rgba[(y * w + x) * 4 + 3] >= options.alpha_threshold;
        }
    }
    opaque.remove_small_islands(options.min_island_area);
    if opaque.is_empty() {
        return Err(MeshError::Empty);
    }

    // 2. `margin` 以内に中身の画素がある画素を領域とする (穴は縮み、近い島はつながる)。
    let region = if margin > 0.0 {
        let d2 = opaque.squared_distance_to(true);
        Mask {
            cells: d2.iter().map(|&d| d <= margin * margin).collect(),
            ..opaque.clone()
        }
    } else {
        opaque.clone()
    };
    // 領域の外までの距離 (内部点を輪郭から離すため)
    let depth: Vec<f32> = region
        .squared_distance_to(false)
        .iter()
        .map(|&d| d.sqrt() as f32)
        .collect();

    // 3. 輪郭 (格子座標 → 画像座標は -pad)
    let offset = -(pad as f64);
    let loops = contour::extract(&region, [offset, offset]);

    // 中身の画素の中心から膨張後の輪郭までの最短距離の下限。簡略化で輪郭が内側に入る量は
    // 許容誤差の最大 2 倍なので、許容誤差をこの半分未満に抑えれば画素の中心は輪郭の内側に残る。
    let clearance = ((margin - 0.5).powi(2) - 0.25)
        .max(0.0)
        .sqrt()
        .max(0.5_f64.sqrt() / 2.0);
    let bounds = Bounds::of(&region, offset);

    let mut scale = 1.0;
    for _ in 0..MAX_ATTEMPTS {
        let edge = edge_spacing * scale;
        let tolerance = (0.25 * edge_spacing).min(0.45 * clearance) * scale;
        let rings: Vec<Vec<P>> = loops
            .iter()
            .filter_map(|ring| contour::simplify(ring, tolerance))
            .map(|ring| contour::resample(&ring, edge))
            .filter(|ring| ring.len() >= 3)
            .collect();
        let boundary_count: usize = rings.iter().map(Vec::len).sum();
        // 外側の 4 隅の番兵を含めて数える
        if boundary_count + 4 <= MAX_VERTICES {
            let interior = if interior_spacing > 0.0 {
                let spacing = interior_spacing * scale;
                let keep_out = 0.5 * edge.min(spacing) + 2.0 * tolerance + 1.5;
                interior_points(&region, &depth, offset, spacing, keep_out, &bounds)
            } else {
                Vec::new()
            };
            if boundary_count + interior.len() + 4 <= MAX_VERTICES {
                let mesh = triangulate::build(&rings, &interior, &bounds.guard(), &opaque, offset);
                return Ok(mesh);
            }
        }
        scale *= SPACING_GROWTH;
    }
    Err(MeshError::TooComplex)
}

fn validate(options: &MeshOptions) -> Result<(), MeshError> {
    if !(options.margin.is_finite() && options.margin >= 0.0) {
        return Err(MeshError::InvalidOptions("margin must be finite and >= 0"));
    }
    if !(options.edge_spacing.is_finite() && options.edge_spacing > 0.0) {
        return Err(MeshError::InvalidOptions(
            "edge_spacing must be finite and > 0",
        ));
    }
    if !(options.interior_spacing.is_finite() && options.interior_spacing >= 0.0) {
        return Err(MeshError::InvalidOptions(
            "interior_spacing must be finite and >= 0",
        ));
    }
    Ok(())
}

/// 領域の画素の中心を囲む矩形 (画像座標)。
#[derive(Debug, Clone, Copy)]
struct Bounds {
    min: P,
    max: P,
}

impl Bounds {
    fn of(mask: &Mask, offset: f64) -> Self {
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for y in 0..mask.height {
            for x in 0..mask.width {
                if mask.get(x, y) {
                    let p = [x as f64 + 0.5 + offset, y as f64 + 0.5 + offset];
                    min = [min[0].min(p[0]), min[1].min(p[1])];
                    max = [max[0].max(p[0]), max[1].max(p[1])];
                }
            }
        }
        Self { min, max }
    }

    /// 三角形分割の凸包がすべての画素を含むように置く、輪郭より外側の 4 点。
    fn guard(&self) -> [P; 4] {
        let (a, b) = (
            [self.min[0] - 2.0, self.min[1] - 2.0],
            [self.max[0] + 2.0, self.max[1] + 2.0],
        );
        [a, [b[0], a[1]], b, [a[0], b[1]]]
    }
}

/// 六角格子 (行ごとに半間隔ずらした格子) の点のうち、領域の境界から `keep_out` 以上離れたもの。
/// 格子は領域の外接矩形の中心に合わせる (左右・上下の余りが対称になる)。
fn interior_points(
    region: &Mask,
    depth: &[f32],
    offset: f64,
    spacing: f64,
    keep_out: f64,
    bounds: &Bounds,
) -> Vec<P> {
    let row_step = spacing * 3f64.sqrt() / 2.0;
    let center = [
        (bounds.min[0] + bounds.max[0]) / 2.0,
        (bounds.min[1] + bounds.max[1]) / 2.0,
    ];
    let rows = ((bounds.max[1] - center[1]) / row_step).floor() as i64;
    let cols = ((bounds.max[0] - center[0]) / spacing).floor() as i64 + 1;
    let mut out = Vec::new();
    for row in -rows..=rows {
        let y = center[1] + row as f64 * row_step;
        let shift = if row % 2 == 0 { 0.0 } else { spacing / 2.0 };
        for col in -cols..=cols {
            let x = center[0] + shift + col as f64 * spacing;
            let (gx, gy) = ((x - offset).floor(), (y - offset).floor());
            if gx < 0.0 || gy < 0.0 {
                continue;
            }
            let (gx, gy) = (gx as usize, gy as usize);
            if gx < region.width
                && gy < region.height
                && f64::from(depth[gy * region.width + gx]) >= keep_out
            {
                out.push(contour::round_f32([x, y]));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
