//! 法線マップの生成 (決定的な方法。`docs/ai-integration.md#法線マップの生成`、ADR 0015)。
//!
//! テクスチャのアルファの形から「ふくらみ」の高さを作り、その勾配を法線にする。
//! アルファがしきい値以上の画素を形とし、形の外 (画像の外も含む) の高さを 0 とする。
//!
//! - **形全体をふくらませる (`radius` なし、既定):** 形の中で `∇²u = −4` (形の外で `u = 0`) を解き、
//!   `√u` を高さにする。円のアルファからは半球 (`u = R² − r²`)、帯からは断面が半円の筒になり、
//!   ほかの形でもなめらかにふくらむ (輪郭からの距離と違い、形の芯に折れ目ができない)。
//!   解き方はマルチグリッド法 (決定的)。
//! - **縁だけを丸める (`radius` あり):** 輪郭からの距離 (ユークリッド距離変換) を `radius` で割った `t`
//!   (1 で頭打ち) を、円弧の断面 `√(1 − (1 − t)²)` で高さにする。中は平ら。
//!
//! 高さの尺度は `strength` 倍 (1 で断面が円、0 で平ら)。高さを小さなガウスぼかしでならし
//! (画素の段差を消す)、中心差分の勾配から法線を作る。ならしと勾配には形の中の画素だけを使う
//! (形の外の高さは意味を持たない)。
//!
//! 法線マップの向きは `docs/spec/rendering.md#ライティング` に従う (x が画像の右、y が画像の **上**、
//! z が手前。`c = (n + 1) / 2` の RGB)。形の外の画素は平ら (`(0, 0, 1)`) にする。

/// ふくらみの方程式を解くマルチグリッドの V サイクルの回数。
const INFLATE_CYCLES: usize = 8;
/// V サイクルの前後で掛ける緩和の回数。
const INFLATE_SMOOTHING: usize = 3;
/// 最も粗い格子で掛ける緩和の回数。
const INFLATE_COARSE_ITERATIONS: usize = 64;
/// 緩和 (SOR) の係数。
const INFLATE_OMEGA: f32 = 1.2;
/// 粗い格子の一辺の下限 (これより小さくしない)。
const INFLATE_COARSEST: usize = 4;

/// 法線マップの生成のエラー。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum NormalMapError {
    /// 画像の大きさが 0。
    #[error("image is empty ({width}x{height})")]
    EmptyImage { width: u32, height: u32 },
    /// 画素の数が大きさと合わない。
    #[error("image is {width}x{height} but has {found} values, expected {expected}")]
    SizeMismatch {
        width: u32,
        height: u32,
        expected: usize,
        found: usize,
    },
    /// 設定の値が範囲の外 (`field` はフィールドの名前)。
    #[error("invalid option {field} = {value}")]
    InvalidOption { field: &'static str, value: f32 },
}

/// ふくらみの設定。パーツ (テクスチャ) ごとに変えられる。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BulgeOptions {
    /// ふくらみの強さ (高さと幅の比)。1 で断面が円 (円のアルファから半球)、0 で平ら。0 以上。
    pub strength: f32,
    /// 輪郭からふくらみが頭打ちになるまでの幅 (画素)。縁だけが丸まり、中は平らになる。
    /// `None` なら形全体を 1 つのふくらみにする。0 より大きい。
    pub radius: Option<f32>,
    /// 高さをならすガウスぼかしの標準偏差 (画素)。0 ならならさない。
    pub smoothing: f32,
    /// 形とみなすアルファの下限 (0〜255)。
    pub alpha_threshold: u8,
}

impl Default for BulgeOptions {
    fn default() -> Self {
        Self {
            strength: 1.0,
            radius: None,
            smoothing: 1.0,
            alpha_threshold: 128,
        }
    }
}

impl BulgeOptions {
    fn check(&self) -> Result<(), NormalMapError> {
        let bad = |field, value: f32| Err(NormalMapError::InvalidOption { field, value });
        if !(self.strength.is_finite() && self.strength >= 0.0) {
            return bad("strength", self.strength);
        }
        if let Some(r) = self.radius
            && !(r.is_finite() && r > 0.0)
        {
            return bad("radius", r);
        }
        if !(self.smoothing.is_finite() && self.smoothing >= 0.0) {
            return bad("smoothing", self.smoothing);
        }
        Ok(())
    }
}

/// 画素ごとの高さ (画素単位、手前が正) と、形 (法線を作る画素) の印。並びは行優先 (x が速い)。
#[derive(Debug, Clone, PartialEq)]
pub struct HeightField {
    pub width: u32,
    pub height: u32,
    pub values: Vec<f32>,
    /// 形の中の画素か。形の外の法線は `(0, 0, 1)` にする。
    pub mask: Vec<bool>,
}

impl HeightField {
    /// 高さ 0 の平らな場。
    pub fn flat(width: u32, height: u32, mask: Vec<bool>) -> Result<Self, NormalMapError> {
        let n = pixel_count(width, height, mask.len())?;
        Ok(Self {
            width,
            height,
            values: vec![0.0; n],
            mask,
        })
    }

    /// 高さを画素ごとに足す (大きさが同じこと)。
    pub fn add(&mut self, other: &HeightField) -> Result<(), NormalMapError> {
        if (other.width, other.height) != (self.width, self.height) {
            return Err(NormalMapError::SizeMismatch {
                width: self.width,
                height: self.height,
                expected: self.values.len(),
                found: other.values.len(),
            });
        }
        for (v, o) in self.values.iter_mut().zip(&other.values) {
            *v += o;
        }
        Ok(())
    }

    /// 形の中の高さをガウスぼかしでならす (`sigma` は画素単位。0 以下なら何もしない)。
    /// 形の外の画素は重みに数えない (正規化した畳み込み) ので、輪郭で外の値に引きずられない。
    pub fn smooth(&mut self, sigma: f32) {
        let (w, h) = (self.width as usize, self.height as usize);
        let weight: Vec<f32> = self.mask.iter().map(|&m| f32::from(u8::from(m))).collect();
        let mut sum: Vec<f32> = self
            .values
            .iter()
            .zip(&weight)
            .map(|(v, m)| v * m)
            .collect();
        let mut total = weight;
        gaussian_blur(&mut sum, w, h, sigma);
        gaussian_blur(&mut total, w, h, sigma);
        for (i, v) in self.values.iter_mut().enumerate() {
            if self.mask[i] && total[i] > 0.0 {
                *v = sum[i] / total[i];
            }
        }
    }

    /// 勾配から法線マップを作る。
    pub fn to_normal_map(&self) -> NormalMap {
        let (w, h) = (self.width as usize, self.height as usize);
        let at = |x: usize, y: usize| self.values[y * w + x];
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                if !self.mask[y * w + x] {
                    rgba.extend(encode_normal([0.0, 0.0, 1.0]));
                    continue;
                }
                // 中心差分。隣が形の外 (か画像の外) なら片側差分にし、両側とも外なら 0。
                let inside = |x: usize, y: usize| self.mask[y * w + x];
                let slope = |a: Option<(usize, usize)>, b: Option<(usize, usize)>| {
                    let a = a.filter(|&(x, y)| inside(x, y)).unwrap_or((x, y));
                    let b = b.filter(|&(x, y)| inside(x, y)).unwrap_or((x, y));
                    let span = (a.0.abs_diff(b.0) + a.1.abs_diff(b.1)) as f32;
                    if span > 0.0 {
                        (at(b.0, b.1) - at(a.0, a.1)) / span
                    } else {
                        0.0
                    }
                };
                let dx = slope(
                    x.checked_sub(1).map(|x| (x, y)),
                    (x + 1 < w).then_some((x + 1, y)),
                );
                let dy = slope(
                    y.checked_sub(1).map(|y| (x, y)),
                    (y + 1 < h).then_some((x, y + 1)),
                );
                // 面の法線は画像の座標 (y が下向き) で (−∂h/∂x, −∂h/∂y, 1)。法線マップの y は上向きなので
                // y の符号を反転する。
                rgba.extend(encode_normal([-dx, dy, 1.0]));
            }
        }
        NormalMap {
            width: self.width,
            height: self.height,
            rgba,
        }
    }
}

/// 法線マップの画像 (RGBA8、アルファは 255)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalMap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl NormalMap {
    /// 画素 `(x, y)` の法線 (単位ベクトルに直したもの)。範囲の外なら `None`。
    pub fn normal(&self, x: u32, y: u32) -> Option<[f32; 3]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        let c = self.rgba.get(i..i + 3)?;
        Some(decode_normal([c[0], c[1], c[2]]))
    }
}

/// 法線を法線マップの色にする (`c = (n / |n| + 1) / 2`、アルファは 255)。長さ 0 なら `(0, 0, 1)`。
pub fn encode_normal(n: [f32; 3]) -> [u8; 4] {
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    let n = if len > 0.0 && len.is_finite() {
        n.map(|v| v / len)
    } else {
        [0.0, 0.0, 1.0]
    };
    let c = n.map(|v| ((v * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8);
    [c[0], c[1], c[2], 255]
}

/// 法線マップの色を単位ベクトルにする (長さ 0 なら `(0, 0, 1)`)。
pub fn decode_normal(c: [u8; 3]) -> [f32; 3] {
    let n = c.map(|v| f32::from(v) / 255.0 * 2.0 - 1.0);
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len > 0.0 {
        n.map(|v| v / len)
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// アルファの形から、ふくらみの高さを作る。`rgba` は RGBA8 (ストレートでも乗算済みでもよい。アルファだけを読む)。
///
/// ならし (`smoothing`) はまだ掛けない ([`bulge_normal_map`] が掛ける)。
pub fn bulge_height(
    width: u32,
    height: u32,
    rgba: &[u8],
    options: &BulgeOptions,
) -> Result<HeightField, NormalMapError> {
    options.check()?;
    let n = pixel_count(width, height, rgba.len() / 4)?;
    if rgba.len() != n * 4 {
        return Err(NormalMapError::SizeMismatch {
            width,
            height,
            expected: n * 4,
            found: rgba.len(),
        });
    }
    let mask: Vec<bool> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[3] >= options.alpha_threshold.max(1))
        .collect();
    let (w, h) = (width as usize, height as usize);
    let values = match options.radius {
        None => inflate(&mask, w, h)
            .into_iter()
            .map(|u| options.strength * u.max(0.0).sqrt())
            .collect(),
        Some(radius) => {
            let scale = options.strength * radius;
            distance_to_outside(&mask, w, h)
                .into_iter()
                .map(|d| {
                    if d <= 0.0 {
                        return 0.0;
                    }
                    let t = (d / radius).min(1.0);
                    scale * (1.0 - (1.0 - t) * (1.0 - t)).sqrt()
                })
                .collect()
        }
    };
    Ok(HeightField {
        width,
        height,
        values,
        mask,
    })
}

/// アルファの形から、ふくらみの法線マップを作る (高さ → ならす → 勾配)。
pub fn bulge_normal_map(
    width: u32,
    height: u32,
    rgba: &[u8],
    options: &BulgeOptions,
) -> Result<NormalMap, NormalMapError> {
    let mut field = bulge_height(width, height, rgba, options)?;
    field.smooth(options.smoothing);
    Ok(field.to_normal_map())
}

/// `width × height` の画素の数。0 なら [`NormalMapError::EmptyImage`]、`found` と違えば
/// [`NormalMapError::SizeMismatch`]。
fn pixel_count(width: u32, height: u32, found: usize) -> Result<usize, NormalMapError> {
    if width == 0 || height == 0 {
        return Err(NormalMapError::EmptyImage { width, height });
    }
    let expected = width as usize * height as usize;
    if found != expected {
        return Err(NormalMapError::SizeMismatch {
            width,
            height,
            expected,
            found,
        });
    }
    Ok(expected)
}

/// 形の中で `∇²u = −4` (形の外と画像の外で `u = 0`) を解く。形の外は 0。
/// 値は画素の二乗の単位 (円なら `R² − r²`)。
///
/// マルチグリッド法の V サイクル ([`INFLATE_CYCLES`] 回) で解く。粗い格子の画素は、もとの 4 画素の
/// 2 つ以上が形の中なら形の中とする。
fn inflate(mask: &[bool], w: usize, h: usize) -> Vec<f32> {
    let mut grids = vec![Grid {
        mask: mask.to_vec(),
        w,
        h,
    }];
    while let Some(g) = grids.last()
        && g.w.min(g.h) / 2 >= INFLATE_COARSEST
    {
        let coarse = g.coarsen();
        grids.push(coarse);
    }
    let mut u = vec![0.0; w * h];
    let f = vec![4.0; w * h];
    for _ in 0..INFLATE_CYCLES {
        v_cycle(&grids, &mut u, &f, 1.0);
    }
    for (v, &m) in u.iter_mut().zip(mask) {
        if !m {
            *v = 0.0;
        }
    }
    u
}

/// マルチグリッドの 1 つの格子 (形の印と大きさ)。
struct Grid {
    mask: Vec<bool>,
    w: usize,
    h: usize,
}

impl Grid {
    fn coarsen(&self) -> Grid {
        let (w, h) = (self.w.div_ceil(2), self.h.div_ceil(2));
        let mask = (0..w * h)
            .map(|i| {
                self.children(i % w, i / w)
                    .filter(|&j| self.mask[j])
                    .count()
                    >= 2
            })
            .collect();
        Grid { mask, w, h }
    }

    /// 粗い格子の画素 `(x, y)` に当たる、この格子の画素 (最大 4 つ)。
    fn children(&self, x: usize, y: usize) -> impl Iterator<Item = usize> + '_ {
        [(0, 0), (1, 0), (0, 1), (1, 1)]
            .into_iter()
            .map(move |(dx, dy)| (x * 2 + dx, y * 2 + dy))
            .filter(|&(x, y)| x < self.w && y < self.h)
            .map(|(x, y)| y * self.w + x)
    }

    /// 形の中の画素の上下左右の値の和 (形の外と画像の外は 0)。
    fn neighbors(&self, u: &[f32], i: usize) -> f32 {
        let (x, y) = (i % self.w, i / self.w);
        let mut sum = 0.0;
        if x > 0 {
            sum += u[i - 1];
        }
        if x + 1 < self.w {
            sum += u[i + 1];
        }
        if y > 0 {
            sum += u[i - self.w];
        }
        if y + 1 < self.h {
            sum += u[i + self.w];
        }
        sum
    }

    /// `−∇²u = f` (格子の間隔 `s`) の SOR を 1 回掛ける。形の外は 0 のまま。
    fn relax(&self, u: &mut [f32], f: &[f32], s: f32) {
        for i in 0..u.len() {
            if self.mask[i] {
                let target = (self.neighbors(u, i) + f[i] * s * s) / 4.0;
                u[i] += INFLATE_OMEGA * (target - u[i]);
            } else {
                u[i] = 0.0;
            }
        }
    }
}

/// `−∇²u = f` の V サイクル。`grids[0]` が `u` の格子、`s` はその間隔。
fn v_cycle(grids: &[Grid], u: &mut [f32], f: &[f32], s: f32) {
    let grid = &grids[0];
    let Some(coarse) = grids.get(1) else {
        for _ in 0..INFLATE_COARSE_ITERATIONS {
            grid.relax(u, f, s);
        }
        return;
    };
    for _ in 0..INFLATE_SMOOTHING {
        grid.relax(u, f, s);
    }
    // 残差 r = f + ∇²u を粗い格子へ (子の平均)。
    let residual: Vec<f32> = (0..u.len())
        .map(|i| {
            if grid.mask[i] {
                f[i] + (grid.neighbors(u, i) - 4.0 * u[i]) / (s * s)
            } else {
                0.0
            }
        })
        .collect();
    let rc: Vec<f32> = (0..coarse.w * coarse.h)
        .map(|i| {
            let (sum, n) = grid
                .children(i % coarse.w, i / coarse.w)
                .filter(|&j| grid.mask[j])
                .fold((0.0, 0), |(sum, n), j| (sum + residual[j], n + 1));
            if n > 0 { sum / n as f32 } else { 0.0 }
        })
        .collect();
    let mut e = vec![0.0; rc.len()];
    v_cycle(&grids[1..], &mut e, &rc, s * 2.0);
    // 補正を細かい格子へ (双線形)。
    for (i, v) in u.iter_mut().enumerate() {
        if grid.mask[i] {
            let x = ((i % grid.w) as f32 - 0.5) / 2.0;
            let y = ((i / grid.w) as f32 - 0.5) / 2.0;
            *v += sample_bilinear(&e, coarse.w, coarse.h, x, y);
        }
    }
    for _ in 0..INFLATE_SMOOTHING {
        grid.relax(u, f, s);
    }
}

/// 格子 `values` (`w × h`、画素の中心が整数座標) の双線形補間。端の外は端の値。
fn sample_bilinear(values: &[f32], w: usize, h: usize, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, (w - 1) as f32);
    let y = y.clamp(0.0, (h - 1) as f32);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |x: usize, y: usize| values[y * w + x];
    let top = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * fx;
    let bottom = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * fx;
    top + (bottom - top) * fy
}

/// 形の中の画素から、形の外 (画像の外を含む) の最も近い画素の中心までの距離から 0.5 を引いたもの
/// (輪郭までのおよその距離)。形の外は 0。
fn distance_to_outside(mask: &[bool], w: usize, h: usize) -> Vec<f32> {
    // 画像の外を形の外にするため、1 画素ずつ広げた格子で距離変換する。
    let (pw, ph) = (w + 2, h + 2);
    let far = ((pw * pw + ph * ph) as f32) * 2.0;
    let mut grid = vec![0.0f32; pw * ph];
    for y in 0..h {
        for x in 0..w {
            if mask[y * w + x] {
                grid[(y + 1) * pw + x + 1] = far;
            }
        }
    }
    let mut line = Vec::new();
    let mut out = Vec::new();
    // 列ごと、行ごとに 1 次元の二乗距離変換を掛ける (Felzenszwalb と Huttenlocher の方法)。
    for x in 0..pw {
        line.clear();
        line.extend((0..ph).map(|y| grid[y * pw + x]));
        squared_distance_1d(&line, &mut out);
        for (y, &v) in out.iter().enumerate() {
            grid[y * pw + x] = v;
        }
    }
    for y in 0..ph {
        line.clear();
        line.extend_from_slice(&grid[y * pw..(y + 1) * pw]);
        squared_distance_1d(&line, &mut out);
        grid[y * pw..(y + 1) * pw].copy_from_slice(&out);
    }
    let mut distance = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            if mask[y * w + x] {
                distance[y * w + x] = grid[(y + 1) * pw + x + 1].sqrt() - 0.5;
            }
        }
    }
    distance
}

/// 1 次元の二乗距離変換: `out[q] = min_p (f[p] + (q − p)²)`。
fn squared_distance_1d(f: &[f32], out: &mut Vec<f32>) {
    let n = f.len();
    out.clear();
    out.resize(n, 0.0);
    // 下側の包絡線をなす放物線の頂点 `v` と、その区間の境 `z`。
    let mut v = vec![0usize; n];
    let mut z = vec![0.0f32; n + 1];
    let mut k = 0;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;
    let intersect = |q: usize, p: usize| {
        let (qf, pf) = (q as f32, p as f32);
        ((f[q] + qf * qf) - (f[p] + pf * pf)) / (2.0 * (qf - pf))
    };
    for q in 1..n {
        let mut s = intersect(q, v[k]);
        while s <= z[k] {
            k -= 1;
            s = intersect(q, v[k]);
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f32::INFINITY;
    }
    k = 0;
    for (q, slot) in out.iter_mut().enumerate() {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let d = q as f32 - v[k] as f32;
        *slot = d * d + f[v[k]];
    }
}

/// 分離したガウスぼかし (`sigma` は画素単位。3σ で打ち切る)。画像の外は端の値とみなす。
fn gaussian_blur(values: &mut [f32], w: usize, h: usize, sigma: f32) {
    if sigma <= 0.0 || w == 0 || h == 0 {
        return;
    }
    let r = (sigma * 3.0).ceil() as isize;
    let kernel: Vec<f32> = (-r..=r)
        .map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp())
        .collect();
    let sum: f32 = kernel.iter().sum();
    let kernel: Vec<f32> = kernel.iter().map(|k| k / sum).collect();
    let mut tmp = vec![0.0; values.len()];
    let clamp = |i: isize, n: usize| i.clamp(0, n as isize - 1) as usize;
    for y in 0..h {
        for x in 0..w {
            tmp[y * w + x] = kernel
                .iter()
                .enumerate()
                .map(|(k, c)| c * values[y * w + clamp(x as isize + k as isize - r, w)])
                .sum();
        }
    }
    for y in 0..h {
        for x in 0..w {
            values[y * w + x] = kernel
                .iter()
                .enumerate()
                .map(|(k, c)| c * tmp[clamp(y as isize + k as isize - r, h) * w + x])
                .sum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_dimensional_transform_finds_nearest_zero() {
        let far = 1e6;
        let mut out = Vec::new();
        squared_distance_1d(&[0.0, far, far, far, 0.0, far], &mut out);
        assert_eq!(out, vec![0.0, 1.0, 4.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn distance_counts_the_image_border_as_outside() {
        // 3x3 がすべて形なら、中央は外 (画像の外) の画素の中心から 2、端は 1。
        let d = distance_to_outside(&[true; 9], 3, 3);
        assert!((d[4] - 1.5).abs() < 1e-6, "{d:?}");
        assert!((d[0] - 0.5).abs() < 1e-6, "{d:?}");
        let d = distance_to_outside(&[false, true, true], 3, 1);
        assert_eq!(d, vec![0.0, 0.5, 0.5]);
    }

    #[test]
    fn inflation_of_a_disc_is_a_paraboloid() {
        // 半径 40 の円: u ≈ R² − r² (境界は形の外の画素の中心なので、R は少し大きく出る)。
        let n = 128;
        let c = n as f32 / 2.0;
        let mask: Vec<bool> = (0..n * n)
            .map(|i| {
                let (x, y) = ((i % n) as f32 + 0.5 - c, (i / n) as f32 + 0.5 - c);
                x.hypot(y) <= 40.0
            })
            .collect();
        let u = inflate(&mask, n, n);
        let at = |x: usize, y: usize| u[y * n + x];
        let center = at(64, 64);
        assert!((center.sqrt() - 40.5).abs() < 1.5, "{}", center.sqrt());
        for r in [10usize, 20, 30] {
            let expected = center - (r * r) as f32;
            for v in [at(64 + r, 64), at(64 - r, 64), at(64, 64 + r)] {
                assert!(
                    (v - expected).abs() < center * 0.03,
                    "r {r}: {v} vs {expected}"
                );
            }
        }
        assert_eq!(at(0, 0), 0.0);
    }

    #[test]
    fn encoding_round_trips() {
        assert_eq!(encode_normal([0.0, 0.0, 1.0]), [128, 128, 255, 255]);
        assert_eq!(encode_normal([0.0, 0.0, 0.0]), [128, 128, 255, 255]);
        assert_eq!(encode_normal([2.0, 0.0, 0.0]), [255, 128, 128, 255]);
        let n = decode_normal([255, 128, 128]);
        assert!((n[0] - 1.0).abs() < 1e-2 && n[1].abs() < 1e-2, "{n:?}");
    }

    #[test]
    fn options_are_checked() {
        let rgba = [255u8; 4];
        for options in [
            BulgeOptions {
                strength: -1.0,
                ..Default::default()
            },
            BulgeOptions {
                radius: Some(0.0),
                ..Default::default()
            },
            BulgeOptions {
                smoothing: f32::NAN,
                ..Default::default()
            },
        ] {
            assert!(matches!(
                bulge_normal_map(1, 1, &rgba, &options),
                Err(NormalMapError::InvalidOption { .. })
            ));
        }
        assert!(matches!(
            bulge_normal_map(2, 1, &rgba, &BulgeOptions::default()),
            Err(NormalMapError::SizeMismatch { .. })
        ));
        assert!(matches!(
            bulge_normal_map(0, 1, &[], &BulgeOptions::default()),
            Err(NormalMapError::EmptyImage { .. })
        ));
    }
}
