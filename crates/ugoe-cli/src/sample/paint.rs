//! 見本の絵を描く小さなラスタライザ (T0710)。
//!
//! 形は符号付きの距離 (内側が負、単位は px) で表し、画素の中心の距離から被覆率を決めて塗る (輪郭がなめらかになる)。
//! 輪郭線は、形を線の幅だけ太らせたものを先に塗って表す。どこでも同じ結果になるよう、計算は四則と平方根だけで行う
//! (三角関数を使わない)。コミットした見本と生成の結果をバイト列で比べるため。

use std::rc::Rc;

use ugoe_core::Vec2;

/// 符号付きの距離で表した形。
#[derive(Clone)]
pub struct Shape(Rc<dyn Fn(Vec2) -> f32>);

impl Shape {
    fn new(f: impl Fn(Vec2) -> f32 + 'static) -> Self {
        Self(Rc::new(f))
    }

    /// 点 `p` の距離 (内側が負)。
    pub fn distance(&self, p: Vec2) -> f32 {
        (self.0)(p)
    }

    pub fn circle(center: Vec2, radius: f32) -> Self {
        Self::new(move |p| p.distance(center) - radius)
    }

    /// 軸に平行な楕円。距離は、陰関数を勾配の大きさで割った近似 (輪郭の近くでは十分に正確)。
    pub fn ellipse(center: Vec2, rx: f32, ry: f32) -> Self {
        Self::new(move |p| {
            let d = p - center;
            let q = Vec2::new(d.x / rx, d.y / ry);
            let len = q.length();
            let gradient = Vec2::new(d.x / (rx * rx), d.y / (ry * ry)).length();
            if gradient < 1e-6 {
                -rx.min(ry)
            } else {
                (len - 1.0) * len / gradient
            }
        })
    }

    /// 角の丸い矩形 (`min`〜`max`、角の半径 `radius`)。
    pub fn rounded_rect(min: Vec2, max: Vec2, radius: f32) -> Self {
        let center = (min + max) * 0.5;
        let half = (max - min) * 0.5 - Vec2::splat(radius);
        Self::new(move |p| {
            let q = (p - center).abs() - half;
            q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - radius
        })
    }

    /// 多角形 (頂点の向きは問わない)。
    pub fn polygon(points: &[Vec2]) -> Self {
        let points = points.to_vec();
        Self::new(move |p| {
            let mut distance = f32::INFINITY;
            let mut inside = false;
            let mut j = points.len() - 1;
            for i in 0..points.len() {
                let (a, b) = (points[j], points[i]);
                distance = distance.min(segment_distance(p, a, b));
                if (b.y > p.y) != (a.y > p.y) && p.x < (a.x - b.x) * (p.y - b.y) / (a.y - b.y) + b.x
                {
                    inside = !inside;
                }
                j = i;
            }
            if inside { -distance } else { distance }
        })
    }

    /// `a` から `b` への線分を半径 `radius` で太らせた形。
    pub fn capsule(a: Vec2, b: Vec2, radius: f32) -> Self {
        Self::new(move |p| segment_distance(p, a, b) - radius)
    }

    /// 点の列を通る、太さが変わる筆の跡 (隣り合う点の間は半径を線形に変える円の和で近似する)。
    pub fn stroke(points: &[(Vec2, f32)]) -> Self {
        let mut circles = Vec::new();
        for pair in points.windows(2) {
            let ((a, ra), (b, rb)) = (pair[0], pair[1]);
            let steps = (a.distance(b) / 2.0).ceil().max(1.0) as usize;
            for k in 0..steps {
                let t = k as f32 / steps as f32;
                circles.push((a + (b - a) * t, ra + (rb - ra) * t));
            }
        }
        if let Some(&last) = points.last() {
            circles.push(last);
        }
        Self::new(move |p| {
            circles
                .iter()
                .map(|(c, r)| p.distance(*c) - r)
                .fold(f32::INFINITY, f32::min)
        })
    }

    /// 中心 `center`、半径 `radius`、太さ `2 * half_width` の輪。
    pub fn ring(center: Vec2, radius: f32, half_width: f32) -> Self {
        Self::new(move |p| (p.distance(center) - radius).abs() - half_width)
    }

    /// `y` が `y0` より小さい (上の) 側。
    pub fn above(y0: f32) -> Self {
        Self::new(move |p| p.y - y0)
    }

    /// `y` が `y0` より大きい (下の) 側。
    pub fn below(y0: f32) -> Self {
        Self::new(move |p| y0 - p.y)
    }

    pub fn union(self, other: Shape) -> Self {
        Self::new(move |p| self.distance(p).min(other.distance(p)))
    }

    pub fn intersect(self, other: Shape) -> Self {
        Self::new(move |p| self.distance(p).max(other.distance(p)))
    }

    pub fn subtract(self, other: Shape) -> Self {
        Self::new(move |p| self.distance(p).max(-other.distance(p)))
    }

    /// 外へ `amount` px 太らせる (負なら細らせる)。
    pub fn grow(self, amount: f32) -> Self {
        Self::new(move |p| self.distance(p) - amount)
    }

    /// 形の和。空なら何もない形。
    pub fn union_all(shapes: impl IntoIterator<Item = Shape>) -> Self {
        shapes
            .into_iter()
            .reduce(Shape::union)
            .unwrap_or_else(|| Self::new(|_| f32::INFINITY))
    }
}

/// 点 `p` と線分 `a`〜`b` の距離。
fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

/// 非乗算の RGBA8 の色。
pub type Color = [u8; 4];

/// `#RRGGBB` の不透明な色。
pub const fn rgb(hex: u32) -> Color {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255]
}

/// 色の不透明度を `alpha` にする。
pub const fn with_alpha(color: Color, alpha: u8) -> Color {
    [color[0], color[1], color[2], alpha]
}

/// キャンバスの一部を切り出した画像 (1 枚のテクスチャになる)。画素は乗算済みの RGBA (0〜1)。
pub struct Layer {
    /// 左上の画素の、キャンバス座標での位置。
    origin: Vec2,
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
}

impl Layer {
    /// キャンバス座標の矩形 `min`〜`max` を覆う空の画像。端は整数に丸めて外へ広げる。
    pub fn new(min: Vec2, max: Vec2) -> Self {
        let origin = min.floor();
        let end = max.ceil();
        let width = (end.x - origin.x).max(1.0) as u32;
        let height = (end.y - origin.y).max(1.0) as u32;
        Self {
            origin,
            width,
            height,
            pixels: vec![[0.0; 4]; (width * height) as usize],
        }
    }

    /// 画像が覆うキャンバス座標の矩形 (`min`, `max`)。
    pub fn bounds(&self) -> (Vec2, Vec2) {
        (
            self.origin,
            self.origin + Vec2::new(self.width as f32, self.height as f32),
        )
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// 形 `shape` を色 `color` で上に重ねて塗る。
    pub fn fill(&mut self, shape: &Shape, color: Color) {
        let [r, g, b, a] = color.map(|c| f32::from(c) / 255.0);
        self.each(shape, |pixel, coverage| {
            let alpha = a * coverage;
            let keep = 1.0 - alpha;
            *pixel = [
                r * alpha + pixel[0] * keep,
                g * alpha + pixel[1] * keep,
                b * alpha + pixel[2] * keep,
                alpha + pixel[3] * keep,
            ];
        });
    }

    /// 輪郭線 (色 `line`、幅 `width`) を付けて形を塗る。
    pub fn paint(&mut self, shape: &Shape, fill: Color, line: Color, width: f32) {
        self.fill(&shape.clone().grow(width), line);
        self.fill(shape, fill);
    }

    fn each(&mut self, shape: &Shape, mut f: impl FnMut(&mut [f32; 4], f32)) {
        let (origin, width) = (self.origin, self.width);
        for (i, pixel) in self.pixels.iter_mut().enumerate() {
            let coverage = coverage(shape.distance(pixel_center(origin, width, i)));
            if coverage > 0.0 {
                f(pixel, coverage);
            }
        }
    }

    /// 非乗算の RGBA8 の PNG。
    pub fn png(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
        for [r, g, b, a] in &self.pixels {
            let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            let alpha = byte(*a);
            if alpha == 0 {
                rgba.extend([0, 0, 0, 0]);
            } else {
                rgba.extend([byte(r / a), byte(g / a), byte(b / a), alpha]);
            }
        }
        ugoe_editor_core::texture::encode_rgba_png(self.width, self.height, &rgba)
    }
}

/// 画素 `index` の中心のキャンバス座標。
fn pixel_center(origin: Vec2, width: u32, index: usize) -> Vec2 {
    let x = (index % width as usize) as f32;
    let y = (index / width as usize) as f32;
    origin + Vec2::new(x + 0.5, y + 0.5)
}

/// 距離から被覆率 (輪郭の前後 0.5px で 0〜1 に変わる)。
fn coverage(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_measure_signed_distance() {
        let circle = Shape::circle(Vec2::new(10.0, 10.0), 5.0);
        assert_eq!(circle.distance(Vec2::new(10.0, 10.0)), -5.0);
        assert_eq!(circle.distance(Vec2::new(20.0, 10.0)), 5.0);
        let ellipse = Shape::ellipse(Vec2::ZERO, 20.0, 10.0);
        assert!(ellipse.distance(Vec2::new(20.0, 0.0)).abs() < 1e-4);
        assert!((ellipse.distance(Vec2::new(0.0, 12.0)) - 2.0).abs() < 0.3);
        let square = Shape::polygon(&[
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ]);
        assert_eq!(square.distance(Vec2::new(5.0, 5.0)), -5.0);
        assert_eq!(square.distance(Vec2::new(5.0, 13.0)), 3.0);
        let rect = Shape::rounded_rect(Vec2::ZERO, Vec2::new(10.0, 10.0), 0.0);
        assert_eq!(rect.distance(Vec2::new(5.0, 13.0)), 3.0);
        let ring = Shape::ring(Vec2::ZERO, 10.0, 1.0);
        assert_eq!(ring.distance(Vec2::new(10.0, 0.0)), -1.0);
    }

    #[test]
    fn layers_paint_with_outlines() {
        let mut layer = Layer::new(Vec2::new(0.0, 0.0), Vec2::new(20.0, 20.0));
        let circle = Shape::circle(Vec2::new(10.0, 10.0), 6.0);
        layer.paint(&circle, rgb(0xff0000), rgb(0x000000), 2.0);
        let at = |layer: &Layer, x: usize, y: usize| layer.pixels[y * 20 + x];
        assert_eq!(at(&layer, 10, 10), [1.0, 0.0, 0.0, 1.0]);
        // 輪郭線 (中心から 7px) は黒、その外は透明。
        assert_eq!(at(&layer, 16, 10), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(at(&layer, 19, 10)[3], 0.0);
        let png = layer.png();
        assert_eq!(
            ugoe_editor_core::texture::png_size(&png),
            Some((layer.width(), layer.height()))
        );
    }
}
