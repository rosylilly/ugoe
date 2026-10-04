//! 部位のまとまりを 1 つの形としてふくらませる法線マップ (`docs/ai-integration.md#まとまりでふくらませる`、T0518)。
//!
//! テクスチャごとに別々にふくらませると、顔の上の目・口が平らに浮いたり、首と肩の境目に段差が出たりする
//! (T0506・T0707 のメモ)。そこでまとまりのテクスチャの形 (アルファ) を基準の姿のキャンバスに並べて 1 つの形にし、
//! その形のふくらみの高さを各テクスチャの画素に写してから法線にする。
//!
//! 1. 格子: キャンバスを、まとまりの中で最も細かいテクスチャの 1 画素の大きさ (キャンバスのピクセル) の升目に分ける。
//! 2. 形: テクスチャの画素 (アルファがしきい値以上) が覆う升目 (画素の中心から一辺 `canvas_per_texel` の正方形) を形の中とする。
//! 3. 高さ: 形を [`bulge_height`] でふくらませ、ならす (`radius`・`smoothing` は升目の単位)。
//! 4. 写す: テクスチャの画素の位置で高さを双線形に引き、テクスチャの画素の単位に直して勾配から法線を作る。
//!    メッシュが覆わない形の中の画素には、隣の値を広げる ([`fill_missing`])。
//! 5. 形の外の画素に、隣の形の中の法線を [`EDGE_PASSES`] 画素だけ広げる ([`NormalMap::extend_edges`])。

use ugoe_core::Vec2;

use super::{
    BulgeOptions, HeightField, NormalMap, NormalMapError, bulge_height, decode_normal,
    encode_normal, sample_bilinear,
};

/// 形の外の画素に、形の中の法線を広げる回数 (テクスチャの画素)。
pub const EDGE_PASSES: usize = 4;
/// 値のない形の中の画素に、隣の値を広げる回数の上限 ([`fill_missing`])。
const FILL_PASSES: usize = 64;
/// まとまりの格子の升目の数の上限 (8192 × 4096)。超えれば確保の前に [`NormalMapError::GridTooLarge`] にする。
///
/// 遠く離れた位置 (キャンバスの外に置いた隠しパーツの頂点など) や極端に小さい `canvas_per_texel` で、
/// 格子が確保できないほど大きくならないように。ふくらませる間は 1 升あたりおよそ 40 バイトを使う。
pub const MAX_GROUP_CELLS: usize = 1 << 25;

/// まとまりの 1 枚のテクスチャと、その画素の基準の姿のキャンバスでの位置。
#[derive(Debug, Clone, Copy)]
pub struct PlacedTexture<'a> {
    pub width: u32,
    pub height: u32,
    /// RGBA8 (アルファだけを読む)。
    pub rgba: &'a [u8],
    /// 画素ごとの、画素の中心のキャンバス座標 (ピクセル。画素 `i` が `[i, i + 1)` を覆う座標)。並びは行優先。
    /// `None` はメッシュが覆わない画素。
    pub positions: &'a [Option<Vec2>],
    /// テクスチャの 1 画素がキャンバスの何ピクセルに当たるか (0 より大きい)。
    pub canvas_per_texel: f32,
}

impl PlacedTexture<'_> {
    fn check(&self) -> Result<(), NormalMapError> {
        let n = super::pixel_count(self.width, self.height, self.positions.len())?;
        if self.rgba.len() != n * 4 {
            return Err(NormalMapError::SizeMismatch {
                width: self.width,
                height: self.height,
                expected: n * 4,
                found: self.rgba.len(),
            });
        }
        if !(self.canvas_per_texel.is_finite() && self.canvas_per_texel > 0.0) {
            return Err(NormalMapError::InvalidOption {
                field: "canvas_per_texel",
                value: self.canvas_per_texel,
            });
        }
        Ok(())
    }

    fn inside(&self, i: usize, threshold: u8) -> bool {
        self.rgba[i * 4 + 3] >= threshold
    }
}

/// まとまり `members` (と、形にだけ加える `shape_only`) を 1 つの形としてふくらませ、`members` の法線マップ
/// (それぞれのテクスチャと同じ大きさ) を並びのとおりに返す。
///
/// `shape_only` は法線マップを作らないが形を広げる (首の上に顔を加えて、首が形の上端として上を向かないようにする、など)。
/// `options` の `radius`・`smoothing` は格子の升目 (まとまりの中で最も細かいテクスチャの画素) の単位。
pub fn group_bulge_normal_maps(
    members: &[PlacedTexture<'_>],
    shape_only: &[PlacedTexture<'_>],
    options: &BulgeOptions,
) -> Result<Vec<NormalMap>, NormalMapError> {
    Ok(group_bulge_normal_maps_oriented(members, shape_only, &[], options)?.members)
}

/// [`group_bulge_normal_maps_oriented`] の結果。
#[derive(Debug, Clone)]
pub struct GroupNormalMaps {
    /// `members` の法線マップ (並びのとおり)。
    pub members: Vec<NormalMap>,
    /// `oriented` の法線マップ (並びのとおり)。
    pub oriented: Vec<NormalMap>,
}

/// [`group_bulge_normal_maps`] に、ふくらませず向きだけを受けるテクスチャ `oriented` を加えたもの (T0520)。
///
/// `oriented` (顔の上の眼鏡など) は形に加えない。画素の位置がまとまりの形の中なら、その位置のふくらみの高さを受け
/// (まとまりの面の向きになる)、形の外の画素 (顔の外に出る眼鏡のつるなど) は、近い形の中の画素の向きを受ける
/// (輪郭の向きのまま続く。64 画素より遠い画素は、形の中から広げた高さの勾配)。まとまりに形がなければ正面向き。
/// `members` の結果は [`group_bulge_normal_maps`] と同じ。
pub fn group_bulge_normal_maps_oriented(
    members: &[PlacedTexture<'_>],
    shape_only: &[PlacedTexture<'_>],
    oriented: &[PlacedTexture<'_>],
    options: &BulgeOptions,
) -> Result<GroupNormalMaps, NormalMapError> {
    options.check()?;
    for t in members.iter().chain(shape_only).chain(oriented) {
        t.check()?;
    }
    let threshold = options.alpha_threshold.max(1);
    let Some(grid) = GroupField::build(members, shape_only, options, threshold)? else {
        // 形がない (テクスチャがない・どの画素も置かれていない)。平ら。
        let flat = |list: &[PlacedTexture<'_>]| -> Result<Vec<NormalMap>, NormalMapError> {
            list.iter().map(|t| flat_map(t, threshold)).collect()
        };
        return Ok(GroupNormalMaps {
            members: flat(members)?,
            oriented: flat(oriented)?,
        });
    };
    let members = members
        .iter()
        .map(|t| grid.map_for(t, threshold, false))
        .collect::<Result<_, _>>()?;
    let oriented = oriented
        .iter()
        .map(|t| grid.map_for(t, threshold, true))
        .collect::<Result<_, _>>()?;
    Ok(GroupNormalMaps { members, oriented })
}

/// まとまりの形をふくらませた高さ (キャンバスに並べた格子)。
struct GroupField {
    origin: Vec2,
    /// 升目の大きさ (キャンバスのピクセル)。
    cell: f32,
    w: usize,
    h: usize,
    /// 升目ごとの高さ (升目の単位)。
    field: HeightField,
}

impl GroupField {
    /// 形がなければ `None`。
    fn build(
        members: &[PlacedTexture<'_>],
        shape_only: &[PlacedTexture<'_>],
        options: &BulgeOptions,
        threshold: u8,
    ) -> Result<Option<Self>, NormalMapError> {
        let all = || members.iter().chain(shape_only);
        let Some(cell) = all().map(|t| t.canvas_per_texel).reduce(f32::min) else {
            return Ok(None);
        };

        // 格子の範囲 (置かれた画素の正方形を覆い、外に 1 升の余白を取る)。
        let (mut min, mut max) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
        for t in all() {
            let half = Vec2::splat(t.canvas_per_texel * 0.5);
            for p in t.positions.iter().flatten() {
                min = min.min(*p - half);
                max = max.max(*p + half);
            }
        }
        if !(min.x.is_finite() && min.y.is_finite() && max.x.is_finite() && max.y.is_finite()) {
            // どの画素も置かれていない (メッシュが覆わない)。
            return Ok(None);
        }
        let origin = Vec2::new(
            (min.x / cell).floor() * cell - cell,
            (min.y / cell).floor() * cell - cell,
        );
        // 升目の数は f64 で数えて、上限を超えれば (無限大・NaN も) 確保の前に拒む。
        let w = f64::from(((max.x - origin.x) / cell).ceil()) + 1.0;
        let h = f64::from(((max.y - origin.y) / cell).ceil()) + 1.0;
        let cells = w * h;
        if cells.is_nan() || cells > MAX_GROUP_CELLS as f64 {
            return Err(NormalMapError::GridTooLarge {
                width: w,
                height: h,
                max: MAX_GROUP_CELLS,
            });
        }
        // 上限の内なので、どちらの辺も u32 と usize に収まる。
        let (w, h) = (w as usize, h as usize);

        // 形: 画素の正方形が中心を含む升目 (少なくとも画素の中心を含む升目)。
        let mut mask = vec![false; w * h];
        for t in all() {
            let half = t.canvas_per_texel * 0.5;
            for (i, p) in t.positions.iter().enumerate() {
                let Some(p) = p else { continue };
                if !t.inside(i, threshold) {
                    continue;
                }
                let g = (*p - origin) / cell;
                // 升目 `i` の中心は `i + 0.5`。範囲は余白の分だけ内側なので負にならない。
                let span = |c: f32| {
                    let lo = (c - half / cell - 0.5).ceil().max(0.0) as usize;
                    let hi = (c + half / cell - 0.5).ceil().max(0.0) as usize;
                    let at = c.floor().max(0.0) as usize;
                    (lo.min(at), hi.max(at + 1))
                };
                let (x0, x1) = span(g.x);
                let (y0, y1) = span(g.y);
                for y in y0..y1.min(h) {
                    for x in x0..x1.min(w) {
                        mask[y * w + x] = true;
                    }
                }
            }
        }
        let rgba: Vec<u8> = mask
            .iter()
            .flat_map(|&m| [0, 0, 0, if m { 255 } else { 0 }])
            .collect();
        let mut field = bulge_height(
            w as u32,
            h as u32,
            &rgba,
            &BulgeOptions {
                alpha_threshold: 255,
                ..*options
            },
        )?;
        field.smooth(options.smoothing);
        Ok(Some(Self {
            origin,
            cell,
            w,
            h,
            field,
        }))
    }

    /// 位置 `p` (キャンバス) が形の中の升目にあるか。
    fn in_shape(&self, p: Vec2) -> bool {
        let g = (p - self.origin) / self.cell;
        if !(g.x >= 0.0 && g.y >= 0.0) {
            return false;
        }
        let (x, y) = (g.x as usize, g.y as usize);
        x < self.w && y < self.h && self.field.mask[y * self.w + x]
    }

    /// テクスチャ `t` の法線マップ。`oriented` なら、形の外の画素は高さを引かず隣から広げる。
    fn map_for(
        &self,
        t: &PlacedTexture<'_>,
        threshold: u8,
        oriented: bool,
    ) -> Result<NormalMap, NormalMapError> {
        // 升目の単位の高さを、このテクスチャの画素の単位に直す。
        let scale = self.cell / t.canvas_per_texel;
        let values: Vec<Option<f32>> = t
            .positions
            .iter()
            .map(|p| {
                p.filter(|p| !oriented || self.in_shape(*p)).map(|p| {
                    let g = (p - self.origin) / self.cell - Vec2::splat(0.5);
                    sample_bilinear(&self.field.values, self.w, self.h, g.x, g.y) * scale
                })
            })
            .collect();
        let mask: Vec<bool> = (0..values.len()).map(|i| t.inside(i, threshold)).collect();
        // 形の中に置かれた画素 (向きだけを受けるテクスチャで、形の向きをそのまま使う画素)。
        let on_shape: Vec<bool> = values
            .iter()
            .zip(&mask)
            .map(|(v, &m)| m && v.is_some())
            .collect();
        let values = fill_missing(values, &mask, t.width, t.height)?;
        let field = HeightField {
            width: t.width,
            height: t.height,
            values,
            mask,
        };
        let mut map = field.to_normal_map();
        if oriented && on_shape.iter().any(|&m| m) {
            // 形の外の画素は、高さの勾配ではなく、隣の形の中の画素の向きを受ける (輪郭で折れない)。
            // 広げた高さの勾配は、形の縁の高さの違いを傾きとして持ち出してしまう。
            let mut outside = map.clone();
            outside.extend_edges(&on_shape, FILL_PASSES)?;
            for (i, &m) in field.mask.iter().enumerate() {
                if m && !on_shape[i] {
                    map.rgba[i * 4..i * 4 + 4].copy_from_slice(&outside.rgba[i * 4..i * 4 + 4]);
                }
            }
        }
        map.extend_edges(&field.mask, EDGE_PASSES)?;
        Ok(map)
    }
}

/// 正面向きの法線マップ。
fn flat_map(t: &PlacedTexture<'_>, threshold: u8) -> Result<NormalMap, NormalMapError> {
    let mask = (0..t.positions.len())
        .map(|i| t.inside(i, threshold))
        .collect();
    Ok(HeightField::flat(t.width, t.height, mask)?.to_normal_map())
}

/// 値のない形の中の画素に、隣 (上下左右) の値の平均を広げる (64 回まで)。それでも値のない画素は、
/// 値のある画素の最小値 (なければ 0) にする。形の外は 0。
///
/// `values`・`mask` の長さが `width × height` でなければ [`NormalMapError::SizeMismatch`]。
pub fn fill_missing(
    mut values: Vec<Option<f32>>,
    mask: &[bool],
    width: u32,
    height: u32,
) -> Result<Vec<f32>, NormalMapError> {
    super::check_len(width, height, values.len())?;
    super::check_len(width, height, mask.len())?;
    let (w, h) = (width as usize, height as usize);
    for _ in 0..FILL_PASSES {
        let mut next = values.clone();
        let mut changed = false;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !mask[i] || values[i].is_some() {
                    continue;
                }
                let neighbors = [
                    x.checked_sub(1).map(|x| y * w + x),
                    (x + 1 < w).then_some(i + 1),
                    y.checked_sub(1).map(|y| y * w + x),
                    (y + 1 < h).then_some(i + w),
                ];
                let (sum, n) = neighbors
                    .into_iter()
                    .flatten()
                    .filter_map(|j| values[j])
                    .fold((0.0, 0), |(s, n), v| (s + v, n + 1));
                if n > 0 {
                    next[i] = Some(sum / n as f32);
                    changed = true;
                }
            }
        }
        values = next;
        if !changed {
            break;
        }
    }
    let floor = values
        .iter()
        .flatten()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let floor = if floor.is_finite() { floor } else { 0.0 };
    Ok(values
        .into_iter()
        .zip(mask)
        .map(|(v, &m)| if m { v.unwrap_or(floor) } else { 0.0 })
        .collect())
}

impl NormalMap {
    /// 形 (`mask`、画素ごと) の外の画素の法線を、形の中から `passes` 画素だけ広げる (隣の広げた画素の法線の平均)。
    ///
    /// 形の外は透明で見えないが、縮小や描画の双線形の補間で輪郭の画素に混ざるので、正面向き `(0, 0, 1)` のままだと、
    /// 重なった部位の輪郭 (肩の上の腕など) に細い線が見える (T0707)。
    ///
    /// `mask` の長さが `width × height`、`rgba` の長さがその 4 倍でなければ [`NormalMapError::SizeMismatch`]
    /// (何も変えない)。
    pub fn extend_edges(&mut self, mask: &[bool], passes: usize) -> Result<(), NormalMapError> {
        let n = super::check_len(self.width, self.height, mask.len())?;
        if self.rgba.len() != n * 4 {
            return Err(NormalMapError::SizeMismatch {
                width: self.width,
                height: self.height,
                expected: n * 4,
                found: self.rgba.len(),
            });
        }
        let (w, h) = (self.width as usize, self.height as usize);
        let mut filled = mask.to_vec();
        for _ in 0..passes {
            let mut next = filled.clone();
            let mut rgba = self.rgba.clone();
            for y in 0..h {
                for x in 0..w {
                    let i = y * w + x;
                    if filled[i] {
                        continue;
                    }
                    let mut sum = [0.0f32; 3];
                    let mut count = 0;
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                        if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                            continue;
                        }
                        let j = ny as usize * w + nx as usize;
                        if filled[j] {
                            let c = &self.rgba[j * 4..j * 4 + 3];
                            for (s, v) in sum.iter_mut().zip(decode_normal([c[0], c[1], c[2]])) {
                                *s += v;
                            }
                            count += 1;
                        }
                    }
                    if count > 0 {
                        rgba[i * 4..i * 4 + 4].copy_from_slice(&encode_normal(sum));
                        next[i] = true;
                    }
                }
            }
            self.rgba = rgba;
            filled = next;
        }
        Ok(())
    }

    /// `factor` 分の 1 の解像度にする (`factor × factor` 画素の法線の平均を正規化。端は切り上げ)。
    /// 法線は変化がなだらかなので、ファイルを小さくするのに使う (法線マップはテクスチャと同じ UV の範囲に引き伸ばして使われる)。
    pub fn downscale(&self, factor: u32) -> NormalMap {
        let factor = factor.max(1);
        let width = self.width.div_ceil(factor);
        let height = self.height.div_ceil(factor);
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0f32; 3];
                for dy in 0..factor {
                    for dx in 0..factor {
                        if let Some(n) = self.normal(x * factor + dx, y * factor + dy) {
                            for (s, v) in sum.iter_mut().zip(n) {
                                *s += v;
                            }
                        }
                    }
                }
                rgba.extend(encode_normal(sum));
            }
        }
        NormalMap {
            width,
            height,
            rgba,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_spreads_values_into_uncovered_pixels() {
        let values = vec![Some(1.0), None, None, None];
        let mask = [true, true, true, false];
        assert_eq!(
            fill_missing(values, &mask, 4, 1),
            Ok(vec![1.0, 1.0, 1.0, 0.0])
        );
        // 何も覆っていなければ 0。
        assert_eq!(
            fill_missing(vec![None; 2], &[true, true], 2, 1),
            Ok(vec![0.0, 0.0])
        );
    }

    /// 長さが大きさと合わなければ、添字を引く前にエラー。
    #[test]
    fn fill_rejects_mismatched_lengths() {
        assert!(matches!(
            fill_missing(vec![None; 3], &[true; 4], 2, 2),
            Err(NormalMapError::SizeMismatch { found: 3, .. })
        ));
        assert!(matches!(
            fill_missing(vec![None; 4], &[true; 2], 2, 2),
            Err(NormalMapError::SizeMismatch { found: 2, .. })
        ));
    }

    #[test]
    fn extend_edges_rejects_mismatched_lengths() {
        let mut map = NormalMap {
            width: 2,
            height: 2,
            rgba: vec![128; 16],
        };
        let before = map.clone();
        assert!(matches!(
            map.extend_edges(&[true], 1),
            Err(NormalMapError::SizeMismatch { found: 1, .. })
        ));
        map.rgba.truncate(8);
        assert!(matches!(
            map.extend_edges(&[true; 4], 1),
            Err(NormalMapError::SizeMismatch { found: 8, .. })
        ));
        map.rgba = before.rgba.clone();
        assert_eq!(map, before);
    }

    fn opaque_square(size: u32) -> Vec<u8> {
        vec![255; (size * size * 4) as usize]
    }

    fn positions(size: u32, at: impl Fn(u32, u32) -> Vec2) -> Vec<Option<Vec2>> {
        (0..size * size)
            .map(|i| Some(at(i % size, i / size)))
            .collect()
    }

    /// 遠く離れた位置 (キャンバスの外の隠しパーツ) があっても、巨大な格子を確保せずにエラーになる。
    #[test]
    fn far_apart_positions_are_rejected_before_allocating() {
        let rgba = opaque_square(2);
        let near = positions(2, |x, y| Vec2::new(x as f32 + 0.5, y as f32 + 0.5));
        let far = positions(2, |x, y| Vec2::new(x as f32 + 1e7, y as f32 + 1e7));
        let texture = |positions| PlacedTexture {
            width: 2,
            height: 2,
            rgba: &rgba,
            positions,
            canvas_per_texel: 1.0,
        };
        let result = group_bulge_normal_maps(
            &[texture(&near)],
            &[texture(&far)],
            &BulgeOptions::default(),
        );
        assert!(
            matches!(
                result,
                Err(NormalMapError::GridTooLarge {
                    max: MAX_GROUP_CELLS,
                    ..
                })
            ),
            "{result:?}"
        );
    }

    /// 極端に小さい `canvas_per_texel` (升目が細かすぎる) もエラー。升目の数が f32 であふれても (無限大) 同じ。
    #[test]
    fn tiny_canvas_per_texel_is_rejected() {
        let rgba = opaque_square(2);
        let placed = positions(2, |x, y| Vec2::new(x as f32 * 100.0, y as f32 * 100.0));
        for canvas_per_texel in [1e-3, f32::MIN_POSITIVE] {
            let result = group_bulge_normal_maps(
                &[PlacedTexture {
                    width: 2,
                    height: 2,
                    rgba: &rgba,
                    positions: &placed,
                    canvas_per_texel,
                }],
                &[],
                &BulgeOptions::default(),
            );
            assert!(
                matches!(result, Err(NormalMapError::GridTooLarge { .. })),
                "{canvas_per_texel}: {result:?}"
            );
        }
    }

    #[test]
    fn edges_take_the_neighbouring_normal() {
        let mut map = NormalMap {
            width: 3,
            height: 1,
            rgba: [
                encode_normal([1.0, 0.0, 1.0]),
                [128, 128, 255, 255],
                [128, 128, 255, 255],
            ]
            .concat(),
        };
        map.extend_edges(&[true, false, false], 1).unwrap();
        assert_eq!(map.rgba[4..8], encode_normal([1.0, 0.0, 1.0]));
        assert_eq!(map.rgba[8..12], [128, 128, 255, 255]);
    }
}
