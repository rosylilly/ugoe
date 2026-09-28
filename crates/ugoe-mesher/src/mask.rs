//! 二値マスクと距離変換。

/// 二値画像。`true` が「中身」。
#[derive(Debug, Clone)]
pub(crate) struct Mask {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<bool>,
}

impl Mask {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![false; width * height],
        }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> bool {
        self.cells[y * self.width + x]
    }

    pub fn is_empty(&self) -> bool {
        !self.cells.iter().any(|&c| c)
    }

    /// 8 近傍でつながった島のうち、画素数が `min_area` 未満のものを消す。
    pub fn remove_small_islands(&mut self, min_area: u32) {
        if min_area <= 1 {
            return;
        }
        let (w, h) = (self.width, self.height);
        let mut visited = vec![false; w * h];
        let mut stack = Vec::new();
        let mut island = Vec::new();
        for start in 0..w * h {
            if !self.cells[start] || visited[start] {
                continue;
            }
            island.clear();
            visited[start] = true;
            stack.push(start);
            while let Some(i) = stack.pop() {
                island.push(i);
                let (x, y) = (i % w, i / w);
                for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                        let j = ny * w + nx;
                        if self.cells[j] && !visited[j] {
                            visited[j] = true;
                            stack.push(j);
                        }
                    }
                }
            }
            if island.len() < min_area as usize {
                for &i in &island {
                    self.cells[i] = false;
                }
            }
        }
    }

    /// `pred` を満たすセルを「種」とした、各セルの中心から最も近い種の中心までの距離の 2 乗。
    /// 種が 1 つもなければすべて `f64::INFINITY` に近い大きな値になる。
    pub fn squared_distance_to(&self, seed: bool) -> Vec<f64> {
        const FAR: f64 = 1e20;
        let (w, h) = (self.width, self.height);
        let mut grid: Vec<f64> = self
            .cells
            .iter()
            .map(|&c| if c == seed { 0.0 } else { FAR })
            .collect();
        let n = w.max(h);
        let mut buf = Buffers::new(n);
        // 列方向
        for x in 0..w {
            for y in 0..h {
                buf.f[y] = grid[y * w + x];
            }
            buf.transform(h);
            for y in 0..h {
                grid[y * w + x] = buf.d[y];
            }
        }
        // 行方向
        for y in 0..h {
            buf.f[..w].copy_from_slice(&grid[y * w..(y + 1) * w]);
            buf.transform(w);
            grid[y * w..(y + 1) * w].copy_from_slice(&buf.d[..w]);
        }
        grid
    }
}

/// 1 次元の 2 乗距離変換 (Felzenszwalb & Huttenlocher, "Distance Transforms of Sampled
/// Functions", 2012) の作業領域。
struct Buffers {
    f: Vec<f64>,
    d: Vec<f64>,
    v: Vec<usize>,
    z: Vec<f64>,
}

impl Buffers {
    fn new(n: usize) -> Self {
        Self {
            f: vec![0.0; n],
            d: vec![0.0; n],
            v: vec![0; n],
            z: vec![0.0; n + 1],
        }
    }

    /// `f[..n]` の下側包絡線を求め、`d[..n]` に書く。
    fn transform(&mut self, n: usize) {
        let Self { f, d, v, z } = self;
        let mut k = 0usize;
        v[0] = 0;
        z[0] = f64::NEG_INFINITY;
        z[1] = f64::INFINITY;
        for q in 1..n {
            let qf = q as f64;
            let mut s;
            loop {
                let pf = v[k] as f64;
                s = ((f[q] + qf * qf) - (f[v[k]] + pf * pf)) / (2.0 * qf - 2.0 * pf);
                // z[0] は -inf なので k == 0 で必ず止まる
                if s > z[k] {
                    break;
                }
                k -= 1;
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f64::INFINITY;
        }
        k = 0;
        for (q, out) in d[..n].iter_mut().enumerate() {
            let qf = q as f64;
            while z[k + 1] < qf {
                k += 1;
            }
            let p = v[k];
            let dp = qf - p as f64;
            *out = dp * dp + f[p];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_transform_matches_brute_force() {
        let mut mask = Mask::new(7, 5);
        for &(x, y) in &[(1, 1), (5, 3), (6, 0)] {
            mask.cells[y * 7 + x] = true;
        }
        let dt = mask.squared_distance_to(true);
        for y in 0..5 {
            for x in 0..7 {
                let brute = [(1i32, 1i32), (5, 3), (6, 0)]
                    .iter()
                    .map(|&(sx, sy)| ((sx - x) * (sx - x) + (sy - y) * (sy - y)) as f64)
                    .fold(f64::INFINITY, f64::min);
                assert_eq!(dt[(y * 7 + x) as usize], brute, "at ({x}, {y})");
            }
        }
    }

    #[test]
    fn small_islands_are_removed() {
        let mut mask = Mask::new(6, 3);
        // 3 画素の島と 1 画素の島
        for &i in &[0, 1, 6, 5] {
            mask.cells[i] = true;
        }
        mask.remove_small_islands(2);
        assert!(mask.cells[0] && mask.cells[1] && mask.cells[6]);
        assert!(!mask.cells[5]);
    }
}
