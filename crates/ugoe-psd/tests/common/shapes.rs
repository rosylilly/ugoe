//! 単色の図形を塗る小さなラスタライザ (確認用 PSD の絵を描く)。
//!
//! 各画素を 4x4 の点で標本化して被覆率を求め、縁を滑らかにする。

/// 非乗算 RGBA8 の画像。
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// 図形。座標は画素の単位で、画素 (x, y) の中心は (x + 0.5, y + 0.5)。
pub enum Shape {
    Ellipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
    },
    /// 角を半径 `r` で丸めた矩形。
    Rect {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        r: f32,
    },
    /// 多角形 (偶奇規則)。
    Polygon(Vec<(f32, f32)>),
    Union(Vec<Shape>),
    /// 1 つ目から 2 つ目を除いたもの。
    Minus(Box<Shape>, Box<Shape>),
}

impl Shape {
    pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        Self::Ellipse { cx, cy, rx, ry }
    }

    pub fn rect(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Self {
        Self::Rect { x0, y0, x1, y1, r }
    }

    pub fn minus(self, other: Shape) -> Self {
        Self::Minus(Box::new(self), Box::new(other))
    }

    fn contains(&self, x: f32, y: f32) -> bool {
        match self {
            Self::Ellipse { cx, cy, rx, ry } => {
                let (dx, dy) = ((x - cx) / rx, (y - cy) / ry);
                dx * dx + dy * dy <= 1.0
            }
            Self::Rect { x0, y0, x1, y1, r } => {
                if x < *x0 || x > *x1 || y < *y0 || y > *y1 {
                    return false;
                }
                // 角の丸め: 内側に縮めた矩形からの距離が r 以下。
                let dx = (x0 + r - x).max(x - (x1 - r)).max(0.0);
                let dy = (y0 + r - y).max(y - (y1 - r)).max(0.0);
                dx * dx + dy * dy <= r * r
            }
            Self::Polygon(points) => {
                let mut inside = false;
                let mut j = points.len() - 1;
                for i in 0..points.len() {
                    let ((xi, yi), (xj, yj)) = (points[i], points[j]);
                    if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                        inside = !inside;
                    }
                    j = i;
                }
                inside
            }
            Self::Union(shapes) => shapes.iter().any(|s| s.contains(x, y)),
            Self::Minus(a, b) => a.contains(x, y) && !b.contains(x, y),
        }
    }
}

impl Image {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; width as usize * height as usize * 4],
        }
    }

    /// 図形を単色で塗り、今の画素の上に重ねる (通常のブレンド)。
    pub fn fill(&mut self, shape: &Shape, color: [u8; 4]) -> &mut Self {
        const N: usize = 4;
        for y in 0..self.height {
            for x in 0..self.width {
                let mut hits = 0;
                for sy in 0..N {
                    for sx in 0..N {
                        let px = x as f32 + (sx as f32 + 0.5) / N as f32;
                        let py = y as f32 + (sy as f32 + 0.5) / N as f32;
                        hits += usize::from(shape.contains(px, py));
                    }
                }
                if hits == 0 {
                    continue;
                }
                let a = f32::from(color[3]) / 255.0 * hits as f32 / (N * N) as f32;
                let i = (y as usize * self.width as usize + x as usize) * 4;
                let dst = &mut self.rgba[i..i + 4];
                let ab = f32::from(dst[3]) / 255.0;
                let ao = a + ab * (1.0 - a);
                for c in 0..3 {
                    let v = (f32::from(color[c]) * a + f32::from(dst[c]) * ab * (1.0 - a)) / ao;
                    dst[c] = v.round() as u8;
                }
                dst[3] = (ao * 255.0).round() as u8;
            }
        }
        self
    }

    /// 不透明度が 0 でない画素の外接矩形で切り出す: (左, 上, 幅, 高さ, 画素)。
    pub fn crop(&self) -> (i32, i32, u32, u32, Vec<u8>) {
        let w = self.width as usize;
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for (i, px) in self.rgba.as_chunks::<4>().0.iter().enumerate() {
            if px[3] != 0 {
                let (x, y) = (i % w, i / w);
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
        assert!(x0 < x1, "empty image");
        let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
        for y in y0..y1 {
            out.extend_from_slice(&self.rgba[(y * w + x0) * 4..(y * w + x1) * 4]);
        }
        (
            x0 as i32,
            y0 as i32,
            (x1 - x0) as u32,
            (y1 - y0) as u32,
            out,
        )
    }
}
