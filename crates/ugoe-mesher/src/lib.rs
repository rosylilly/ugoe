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

use std::ops::{Bound, RangeBounds};

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
/// `margin` の上限 (px)。作業用の格子は画像の周りに `margin` だけ広がるので、大きすぎる値で
/// 確保が爆発しないように抑える。
const MAX_MARGIN: f32 = 1024.0;
/// 内部点の候補にする六角格子の点の数の上限。これを超える間隔の小さい格子は、点を数え上げる前に
/// 間隔を広げてやり直す (極端に小さい `interior_spacing` で処理が終わらなくならないように)。
const MAX_GRID_POINTS: f64 = 256.0 * MAX_VERTICES as f64;
/// 作業用の格子 (中身の外接矩形に `margin` の余白を付けたもの) のセル数の上限 (16384 x 16384)。
/// 1 セルあたり 20 バイトほど確保するので、これで約 5 GB に抑える。
/// 輪郭の辺 ID (`contour.rs`、セルあたり 2 つの `u32`) があふれないことも、これで保証する。
pub const MAX_GRID_CELLS: usize = 1 << 28;
const _: () = assert!(2 * MAX_GRID_CELLS < u32::MAX as usize);

/// [`generate`] のオプション。既定値と範囲はクレートの README の「オプション」。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshOptions {
    /// この値以上のアルファを「中身」とみなす (0..=255)。
    pub alpha_threshold: u8,
    /// 輪郭を外側に広げる距離 (px、0..=1024)。簡略化で最大半分ほど内側に戻ることがある。
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

/// [`generate`] のエラー。
///
/// 変種を足しても公開 API の破壊的な変更にならないよう `#[non_exhaustive]` にしている
/// (外の `match` は `_` の腕が要る)。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum MeshError {
    /// 中身 (`alpha_threshold` 以上の画素) が 1 つもない。
    #[error("image is fully transparent")]
    Empty,
    /// `rgba` の長さが `width * height * 4` と合わない。
    #[error("rgba length {len} does not match {width}x{height}")]
    BadImage { width: u32, height: u32, len: usize },
    /// オプション `option` の値 `value` が、受け付ける範囲 ([`MeshOption::bounds`]) の外か、有限でない。
    #[error("invalid mesh option: {option} is {value}, but must be finite and in {}", option.range_text())]
    InvalidOptions { option: MeshOption, value: f32 },
    /// 間隔を広げてやり直しても、頂点数が [`MAX_VERTICES`] に収まらない。
    #[error("shape is too complex to fit in 65535 vertices")]
    TooComplex,
    /// 中身の外接矩形に `margin` の余白を付けた作業用の格子 (`width` x `height` セル) が
    /// [`MAX_GRID_CELLS`] を超える。
    #[error("working grid {width}x{height} exceeds {MAX_GRID_CELLS} cells")]
    TooLarge { width: u64, height: u64 },
}

impl MeshError {
    /// 機械が読む種類 (`"empty"` など。バリアントの名前の `snake_case`)。値は **変えない**。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::BadImage { .. } => "bad_image",
            Self::InvalidOptions { .. } => "invalid_options",
            Self::TooComplex => "too_complex",
            Self::TooLarge { .. } => "too_large",
        }
    }
}

/// 範囲を確かめる [`MeshOptions`] のフィールド ([`MeshError::InvalidOptions`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MeshOption {
    Margin,
    EdgeSpacing,
    InteriorSpacing,
}

impl MeshOption {
    /// フィールドの名前 (`"margin"` など。[`MeshOptions`] のフィールド名)。値は **変えない**。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Margin => "margin",
            Self::EdgeSpacing => "edge_spacing",
            Self::InteriorSpacing => "interior_spacing",
        }
    }

    /// 受け付ける値の範囲 (下限と上限)。値はこれに加えて有限でなければならない。
    ///
    /// `margin` は 0 以上 1024 以下、`edge_spacing` は 0 より大きく、`interior_spacing` は 0 以上。
    pub fn bounds(self) -> (Bound<f32>, Bound<f32>) {
        match self {
            Self::Margin => (Bound::Included(0.0), Bound::Included(MAX_MARGIN)),
            Self::EdgeSpacing => (Bound::Excluded(0.0), Bound::Unbounded),
            Self::InteriorSpacing => (Bound::Included(0.0), Bound::Unbounded),
        }
    }

    /// [`MeshOption::bounds`] を区間の記法で書いた文 (`[0, 1024]`・`(0, inf)` など)。言語によらない。
    pub fn range_text(self) -> String {
        let (start, end) = self.bounds();
        let lower = match start {
            Bound::Included(v) => format!("[{v}"),
            Bound::Excluded(v) => format!("({v}"),
            Bound::Unbounded => "(-inf".to_owned(),
        };
        let upper = match end {
            Bound::Included(v) => format!("{v}]"),
            Bound::Excluded(v) => format!("{v})"),
            Bound::Unbounded => "inf)".to_owned(),
        };
        format!("{lower}, {upper}")
    }

    /// `value` を受け付けるか (有限で、[`MeshOption::bounds`] の内側)。NaN は受け付けない。
    fn accepts(self, value: f32) -> bool {
        value.is_finite() && self.bounds().contains(&value)
    }
}

impl std::fmt::Display for MeshOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 非乗算 RGBA8 画像からメッシュを生成する。
///
/// 作業用の格子は中身 (`alpha_threshold` 以上の画素) の外接矩形に `margin` の余白を付けた
/// 大きさで、[`MAX_GRID_CELLS`] を超えるなら [`MeshError::TooLarge`] を返す。
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
    options.validate()?;
    let rect =
        Rect::of_content(rgba, width, height, options.alpha_threshold).ok_or(MeshError::Empty)?;
    generate_in(rgba, width, options, rect)
}

/// `rect` (中身をすべて含む画像の矩形) に余白を付けた格子でメッシュを作る。出力は `rect` によらない。
fn generate_in(
    rgba: &[u8],
    width: u32,
    options: &MeshOptions,
    rect: Rect,
) -> Result<GeneratedMesh, MeshError> {
    let margin = f64::from(options.margin);
    let edge_spacing = f64::from(options.edge_spacing);
    let interior_spacing = f64::from(options.interior_spacing);

    // 1. 二値化。中身の矩形に、膨張した領域が収まる余白を付けた格子で扱う。
    // `margin` は検証済みで 1024 以下
    let pad = margin.ceil() as usize + 2;
    let (gw, gh) = grid_size(&rect, pad)?;
    let mut opaque = Mask::new(gw, gh);
    let w = width as usize;
    for y in 0..rect.height {
        let src = (rect.y + y) * w + rect.x;
        let dst = (y + pad) * gw + pad;
        for x in 0..rect.width {
            opaque.cells[dst + x] = rgba[(src + x) * 4 + 3] >= options.alpha_threshold;
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

    // 3. 輪郭 (格子座標 → 画像座標は矩形の左上 - pad)
    let offset = [rect.x as f64 - pad as f64, rect.y as f64 - pad as f64];
    let loops = contour::extract(&region, offset);

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
        let simplified: Vec<Vec<P>> = loops
            .iter()
            .filter_map(|ring| contour::simplify(ring, tolerance))
            .collect();
        // 再サンプリングした点の数を先に見積もり、収まらなければ点を作らずにやり直す
        let estimate: f64 = simplified
            .iter()
            .map(|ring| contour::resample_count(ring, edge))
            .sum();
        if estimate + 4.0 > MAX_VERTICES as f64 {
            scale *= SPACING_GROWTH;
            continue;
        }
        let rings: Vec<Vec<P>> = simplified
            .iter()
            .map(|ring| contour::resample(ring, edge))
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
                Some(Vec::new())
            };
            if let Some(interior) = interior
                && boundary_count + interior.len() + 4 <= MAX_VERTICES
            {
                let mesh = triangulate::build(&rings, &interior, &bounds.guard(), &opaque, offset);
                return Ok(mesh);
            }
        }
        scale *= SPACING_GROWTH;
    }
    Err(MeshError::TooComplex)
}

impl MeshOptions {
    /// [`generate`] が受け付ける値か確かめる (`margin` は 0 以上 1024 以下、`interior_spacing` は
    /// 0 以上、`edge_spacing` は 0 より大きい有限の値。[`MeshOption::bounds`])。範囲の外なら、
    /// 最初に外れたフィールドとその値を持つ [`MeshError::InvalidOptions`]。
    ///
    /// 呼び出し側 (エディタの取り込みなど) がオプションを先に確かめるときも、これを使う
    /// (検証の規則を 1 か所に保つため)。
    pub fn validate(&self) -> Result<(), MeshError> {
        for (option, value) in [
            (MeshOption::Margin, self.margin),
            (MeshOption::EdgeSpacing, self.edge_spacing),
            (MeshOption::InteriorSpacing, self.interior_spacing),
        ] {
            if !option.accepts(value) {
                return Err(MeshError::InvalidOptions { option, value });
            }
        }
        Ok(())
    }
}

/// 画像の画素の矩形 (左上と大きさ、画素単位)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Rect {
    /// アルファが `threshold` 以上の画素の外接矩形。そのような画素がなければ `None`。
    fn of_content(rgba: &[u8], width: u32, height: u32, threshold: u8) -> Option<Self> {
        let (w, h) = (width as usize, height as usize);
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for y in 0..h {
            let row = rgba[y * w * 4..(y + 1) * w * 4].as_chunks::<4>().0;
            let is_content = |p: &[u8; 4]| p[3] >= threshold;
            let Some(first) = row.iter().position(is_content) else {
                continue;
            };
            let last = row.iter().rposition(is_content).unwrap_or(first);
            x0 = x0.min(first);
            x1 = x1.max(last);
            y0 = y0.min(y);
            y1 = y;
        }
        (y0 <= y1).then(|| Self {
            x: x0,
            y: y0,
            width: x1 - x0 + 1,
            height: y1 - y0 + 1,
        })
    }
}

/// `rect` の周りに `pad` の余白を付けた格子の大きさ。[`MAX_GRID_CELLS`] を超えるなら
/// [`MeshError::TooLarge`]。
fn grid_size(rect: &Rect, pad: usize) -> Result<(usize, usize), MeshError> {
    let (width, height) = (
        rect.width as u64 + 2 * pad as u64,
        rect.height as u64 + 2 * pad as u64,
    );
    if width.saturating_mul(height) > MAX_GRID_CELLS as u64 {
        return Err(MeshError::TooLarge { width, height });
    }
    // 上限より小さいので usize に収まる
    Ok((width as usize, height as usize))
}

/// 領域の画素の中心を囲む矩形 (画像座標)。
#[derive(Debug, Clone, Copy)]
struct Bounds {
    min: P,
    max: P,
}

impl Bounds {
    fn of(mask: &Mask, offset: P) -> Self {
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for y in 0..mask.height {
            for x in 0..mask.width {
                if mask.get(x, y) {
                    let p = [x as f64 + 0.5 + offset[0], y as f64 + 0.5 + offset[1]];
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
/// 格子の点が [`MAX_GRID_POINTS`] を超えるなら `None`。
fn interior_points(
    region: &Mask,
    depth: &[f32],
    offset: P,
    spacing: f64,
    keep_out: f64,
    bounds: &Bounds,
) -> Option<Vec<P>> {
    let row_step = spacing * 3f64.sqrt() / 2.0;
    let center = [
        (bounds.min[0] + bounds.max[0]) / 2.0,
        (bounds.min[1] + bounds.max[1]) / 2.0,
    ];
    let rows = ((bounds.max[1] - center[1]) / row_step).floor();
    let cols = ((bounds.max[0] - center[0]) / spacing).floor() + 1.0;
    if (2.0 * rows + 1.0) * (2.0 * cols + 1.0) > MAX_GRID_POINTS {
        return None;
    }
    // 上で数を抑えたので i64 に収まる
    let (rows, cols) = (rows as i64, cols as i64);
    let mut out = Vec::new();
    for row in -rows..=rows {
        let y = center[1] + row as f64 * row_step;
        let shift = if row % 2 == 0 { 0.0 } else { spacing / 2.0 };
        for col in -cols..=cols {
            let x = center[0] + shift + col as f64 * spacing;
            let (gx, gy) = ((x - offset[0]).floor(), (y - offset[1]).floor());
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
    Some(out)
}

#[cfg(test)]
mod tests;
