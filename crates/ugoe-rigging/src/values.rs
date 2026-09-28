//! 推定で足し引きするキーフォームの数値 (`docs/spec/keyform-inference.md#推定の式`)。

use ugoe_core::{Rgb, Vec2};
use ugoe_model::deformer::{RotationKeyform, WarpKeyform};
use ugoe_model::mesh::{GlueKeyform, MeshKeyform};
use ugoe_model::part::PartKeyform;

/// 推定で足し引きできるキーフォーム。
///
/// 数値のフィールドを決まった順の `f32` の列に平らにして扱う。推定は列どうしの足し算・引き算・
/// 定数倍で行い、結果を基準のセルの複製に書き戻してからクランプする。
/// 真偽値など数値でないフィールドは書き戻さないので、基準のセルの値になる。
pub trait KeyformValues: Clone {
    /// 頂点・制御点の数。1 つの格子の中で揃っていなければならない。形を持たない型は 0。
    fn shape_len(&self) -> usize;

    /// 数値のフィールドを決まった順で `out` の末尾に書く。
    /// 書く数は `shape_len` だけで決まる。
    fn write_values(&self, out: &mut Vec<f32>);

    /// `write_values` と同じ順の値で数値のフィールドを上書きする。
    /// `values` は `shape_len` が同じキーフォームの `write_values` が書いたのと同じ長さ。
    fn read_values(&mut self, values: &[f32]);

    /// `write_values` が書く値のうち、先頭の形の値 (頂点・制御点・回転の原点・角度・拡大率) の個数。
    /// 曲線補間の格子 (`KeyformInterpolation::Smooth`) はこの値だけを曲線で補間し、残りは線形で補間する
    /// (`docs/spec/evaluation.md#曲線補間`)。
    fn smooth_len(&self) -> usize;

    /// 値を有効な範囲にクランプする (不透明度・色・`mask_strength` は 0〜1、拡大率は 0 以上)。
    fn clamp(&mut self);
}

/// `read_values` で値を先頭から順に読む。足りなければ 0 を返す。
struct Reader<'a> {
    values: &'a [f32],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(values: &'a [f32]) -> Self {
        Self { values, pos: 0 }
    }

    fn f32(&mut self) -> f32 {
        let v = self.values.get(self.pos).copied().unwrap_or(0.0);
        self.pos += 1;
        v
    }

    fn vec2(&mut self) -> Vec2 {
        let x = self.f32();
        Vec2::new(x, self.f32())
    }

    fn rgb(&mut self) -> Rgb {
        let r = self.f32();
        let g = self.f32();
        Rgb {
            r,
            g,
            b: self.f32(),
        }
    }
}

fn write_vec2(out: &mut Vec<f32>, v: Vec2) {
    out.extend([v.x, v.y]);
}

fn write_rgb(out: &mut Vec<f32>, c: Rgb) {
    out.extend([c.r, c.g, c.b]);
}

fn clamp_unit(v: &mut f32) {
    *v = v.clamp(0.0, 1.0);
}

fn clamp_rgb(c: &mut Rgb) {
    clamp_unit(&mut c.r);
    clamp_unit(&mut c.g);
    clamp_unit(&mut c.b);
}

impl KeyformValues for MeshKeyform {
    fn smooth_len(&self) -> usize {
        2 * self.positions.len()
    }

    fn shape_len(&self) -> usize {
        self.positions.len()
    }

    fn write_values(&self, out: &mut Vec<f32>) {
        for &p in &self.positions {
            write_vec2(out, p);
        }
        out.extend([self.opacity, self.draw_order]);
        write_rgb(out, self.multiply_color);
        write_rgb(out, self.screen_color);
        out.push(self.mask_strength);
    }

    fn read_values(&mut self, values: &[f32]) {
        let mut r = Reader::new(values);
        for p in &mut self.positions {
            *p = r.vec2();
        }
        self.opacity = r.f32();
        self.draw_order = r.f32();
        self.multiply_color = r.rgb();
        self.screen_color = r.rgb();
        self.mask_strength = r.f32();
    }

    fn clamp(&mut self) {
        clamp_unit(&mut self.opacity);
        clamp_rgb(&mut self.multiply_color);
        clamp_rgb(&mut self.screen_color);
        clamp_unit(&mut self.mask_strength);
    }
}

impl KeyformValues for PartKeyform {
    fn smooth_len(&self) -> usize {
        0
    }

    fn shape_len(&self) -> usize {
        0
    }

    fn write_values(&self, out: &mut Vec<f32>) {
        out.push(self.opacity);
    }

    fn read_values(&mut self, values: &[f32]) {
        self.opacity = Reader::new(values).f32();
    }

    fn clamp(&mut self) {
        clamp_unit(&mut self.opacity);
    }
}

impl KeyformValues for WarpKeyform {
    fn smooth_len(&self) -> usize {
        2 * self.points.len()
    }

    fn shape_len(&self) -> usize {
        self.points.len()
    }

    fn write_values(&self, out: &mut Vec<f32>) {
        for &p in &self.points {
            write_vec2(out, p);
        }
        out.push(self.opacity);
    }

    fn read_values(&mut self, values: &[f32]) {
        let mut r = Reader::new(values);
        for p in &mut self.points {
            *p = r.vec2();
        }
        self.opacity = r.f32();
    }

    fn clamp(&mut self) {
        clamp_unit(&mut self.opacity);
    }
}

impl KeyformValues for RotationKeyform {
    fn smooth_len(&self) -> usize {
        // origin (2)・angle・scale
        4
    }

    fn shape_len(&self) -> usize {
        0
    }

    fn write_values(&self, out: &mut Vec<f32>) {
        write_vec2(out, self.origin);
        out.extend([self.angle, self.scale, self.opacity]);
    }

    fn read_values(&mut self, values: &[f32]) {
        let mut r = Reader::new(values);
        self.origin = r.vec2();
        self.angle = r.f32();
        self.scale = r.f32();
        self.opacity = r.f32();
    }

    fn clamp(&mut self) {
        self.scale = self.scale.max(0.0);
        clamp_unit(&mut self.opacity);
    }
}

impl KeyformValues for GlueKeyform {
    fn smooth_len(&self) -> usize {
        0
    }

    fn shape_len(&self) -> usize {
        0
    }

    fn write_values(&self, out: &mut Vec<f32>) {
        out.push(self.intensity);
    }

    fn read_values(&mut self, values: &[f32]) {
        self.intensity = Reader::new(values).f32();
    }

    /// グルーの強さは 0〜1 (`docs/spec/model.md` のグルー)。
    fn clamp(&mut self) {
        clamp_unit(&mut self.intensity);
    }
}

/// 頂点・制御点の位置を持つキーフォーム (ミラー・深度からの回転で動かす)。
pub(crate) trait Points: KeyformValues {
    fn points(&self) -> &[Vec2];
    fn points_mut(&mut self) -> &mut Vec<Vec2>;
}

impl Points for MeshKeyform {
    fn points(&self) -> &[Vec2] {
        &self.positions
    }

    fn points_mut(&mut self) -> &mut Vec<Vec2> {
        &mut self.positions
    }
}

impl Points for WarpKeyform {
    fn points(&self) -> &[Vec2] {
        &self.points
    }

    fn points_mut(&mut self) -> &mut Vec<Vec2> {
        &mut self.points
    }
}
