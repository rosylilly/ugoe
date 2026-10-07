//! デフォーマのワールド変換 (`docs/spec/evaluation.md` の「3. デフォーマのワールド変換」)。

use ugoe_core::math;
use ugoe_core::{Affine2, Mat2, Vec2};

/// 回転デフォーマの親がワープのときに向きを求める差分の幅 (UV 単位)。
pub const ROTATION_EPSILON: f32 = 0.01;

/// 補間後の回転デフォーマのキーフォーム値。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation {
    pub origin: Vec2,
    /// 度数法、時計回り正。
    pub angle: f32,
    pub scale: f32,
    pub reflect_x: bool,
    pub reflect_y: bool,
}

impl Rotation {
    /// `R(angle) * S`。
    fn linear(&self, angle_deg: f32) -> Mat2 {
        let sx = if self.reflect_x {
            -self.scale
        } else {
            self.scale
        };
        let sy = if self.reflect_y {
            -self.scale
        } else {
            self.scale
        };
        Mat2::from_angle(angle_deg.to_radians()) * Mat2::from_diagonal(Vec2::new(sx, sy))
    }

    /// 局所変換 `L(p) = origin + R(angle) * S * p`。
    pub fn local(&self) -> Affine2 {
        Affine2::from_mat2_translation(self.linear(self.angle), self.origin)
    }
}

/// `lerp(a, b, t) = a + (b - a) * t` (仕様の式どおりの演算順)。
pub(crate) fn lerp(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    a + (b - a) * t
}

/// 局所座標 → キャンバス座標の写像 (1 つのデフォーマのワールド変換)。
#[derive(Debug, Clone, Copy)]
pub enum Transform<'a> {
    /// 親なし。
    Identity,
    /// 回転デフォーマ。
    Affine(Affine2),
    /// ワープデフォーマ。`points` はワールド座標の制御点 (行優先、x が速い)。
    Warp(Warp<'a>),
}

impl Transform<'_> {
    pub fn apply(&self, p: Vec2) -> Vec2 {
        match self {
            Transform::Identity => p,
            Transform::Affine(a) => a.transform_point2(p),
            Transform::Warp(w) => w.map(p),
        }
    }

    /// 点 `p` (局所座標) でのヤコビ行列 (列は局所座標の x・y 方向の微分)。
    pub fn jacobian(&self, p: Vec2) -> Mat2 {
        match self {
            Transform::Identity => Mat2::IDENTITY,
            Transform::Affine(a) => a.matrix2,
            Transform::Warp(w) => w.jacobian(p),
        }
    }

    /// 親がこの変換である回転デフォーマのワールド変換を作る。
    pub fn rotation_child(&self, rotation: &Rotation) -> Affine2 {
        match self {
            Transform::Identity => rotation.local(),
            Transform::Affine(parent) => *parent * rotation.local(),
            Transform::Warp(w) => {
                // 原点と向きだけをワープで変換する (子にワープの伸縮は伝えない)。
                let o = w.map(rotation.origin);
                let up = w.map(rotation.origin + Vec2::new(0.0, -ROTATION_EPSILON)) - o;
                // R(δ) * (0, -1) = (sin δ, -cos δ) が up の向きになる δ。
                let delta = math::atan2(up.x, -up.y).to_degrees();
                Affine2::from_mat2_translation(rotation.linear(rotation.angle + delta), o)
            }
        }
    }
}

/// ワールド座標の制御点を持つワープ格子。
#[derive(Debug, Clone, Copy)]
pub struct Warp<'a> {
    points: &'a [Vec2],
    cols: usize,
    rows: usize,
    /// 範囲外で使うアフィン近似の u 方向・v 方向の軸 (四隅から作る)。
    axis_u: Vec2,
    axis_v: Vec2,
}

impl<'a> Warp<'a> {
    /// `points.len() == (rows + 1) * (cols + 1)`、`rows >= 1`、`cols >= 1` であること
    /// (モデルの検証で確かめる。満たさなければ、足りない制御点を原点とみなす)。
    pub fn new(points: &'a [Vec2], cols: usize, rows: usize) -> Self {
        let mut warp = Self {
            points,
            cols,
            rows,
            axis_u: Vec2::ZERO,
            axis_v: Vec2::ZERO,
        };
        let c00 = warp.point(0, 0);
        let c10 = warp.point(cols, 0);
        let c01 = warp.point(0, rows);
        let c11 = warp.point(cols, rows);
        warp.axis_u = ((c10 - c00) + (c11 - c01)) * 0.5;
        warp.axis_v = ((c01 - c00) + (c11 - c10)) * 0.5;
        warp
    }

    fn point(&self, x: usize, y: usize) -> Vec2 {
        let i = y
            .saturating_mul(self.cols.saturating_add(1))
            .saturating_add(x);
        self.points.get(i).copied().unwrap_or(Vec2::ZERO)
    }

    /// `uv` を含むセル `(i, j)` とセルの中での位置 `(fx, fy)`。範囲外なら端のセル。
    fn cell(&self, uv: Vec2) -> (usize, usize, f32, f32) {
        let gx = uv.x * self.cols as f32;
        let gy = uv.y * self.rows as f32;
        let i = (math::floor(gx).max(0.0) as usize).min(self.cols.saturating_sub(1));
        let j = (math::floor(gy).max(0.0) as usize).min(self.rows.saturating_sub(1));
        (i, j, gx - i as f32, gy - j as f32)
    }

    /// セル単位の双線形写像。`uv` が範囲外なら端のセルの式をそのまま延長する。
    fn bilinear(&self, uv: Vec2) -> Vec2 {
        let (i, j, fx, fy) = self.cell(uv);
        let top = lerp(self.point(i, j), self.point(i + 1, j), fx);
        let bottom = lerp(self.point(i, j + 1), self.point(i + 1, j + 1), fx);
        lerp(top, bottom, fy)
    }

    /// 局所座標 (UV) → ワールド座標。
    ///
    /// [0,1]² の内側は双線形。外側は、境界へクランプした点の双線形の値に、四隅から作った
    /// アフィン近似の軸で境界からのずれを足す (`docs/spec/evaluation.md` のワープデフォーマ)。
    pub fn map(&self, uv: Vec2) -> Vec2 {
        let clamped = uv.clamp(Vec2::ZERO, Vec2::ONE);
        let base = self.bilinear(clamped);
        if clamped == uv {
            return base;
        }
        let d = uv - clamped;
        base + self.axis_u * d.x + self.axis_v * d.y
    }

    /// [`Warp::map`] のヤコビ行列 (列は u 方向・v 方向の微分)。
    ///
    /// 内側はセルの双線形式の微分。セルの境目では [`Warp::map`] と同じく `floor` で選んだセル
    /// (上端・右端では端のセル) の片側の微分になる。外側では、はみ出した方向の列が
    /// アフィン近似の軸 (`A_u`・`A_v`) になり、はみ出していない方向の列は境界上の双線形式の微分になる。
    pub fn jacobian(&self, uv: Vec2) -> Mat2 {
        let clamped = uv.clamp(Vec2::ZERO, Vec2::ONE);
        let (i, j, fx, fy) = self.cell(clamped);
        let du = if clamped.x == uv.x {
            let top = self.point(i + 1, j) - self.point(i, j);
            let bottom = self.point(i + 1, j + 1) - self.point(i, j + 1);
            lerp(top, bottom, fy) * self.cols as f32
        } else {
            self.axis_u
        };
        let dv = if clamped.y == uv.y {
            let left = self.point(i, j + 1) - self.point(i, j);
            let right = self.point(i + 1, j + 1) - self.point(i + 1, j);
            lerp(left, right, fx) * self.rows as f32
        } else {
            self.axis_v
        };
        Mat2::from_cols(du, dv)
    }

    /// 比較用: 範囲外も端のセルの双線形式を延長する方式 (採用しなかった案)。
    #[cfg(test)]
    fn map_edge_bilinear(&self, uv: Vec2) -> Vec2 {
        self.bilinear(uv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f32 = 1e-4;

    fn assert_close(a: Vec2, b: Vec2) {
        assert!(a.abs_diff_eq(b, TOL), "{a} != {b}");
    }

    /// 2x2 セルの格子を `f` で作る。
    fn grid(f: impl Fn(f32, f32) -> Vec2) -> Vec<Vec2> {
        let mut points = Vec::new();
        for y in 0..=2 {
            for x in 0..=2 {
                points.push(f(x as f32 / 2.0, y as f32 / 2.0));
            }
        }
        points
    }

    /// 回転 + せん断を含むアフィン写像。
    fn affine(uv: Vec2) -> Vec2 {
        let m = Mat2::from_cols(Vec2::new(90.0, 40.0), Vec2::new(-30.0, 110.0));
        Vec2::new(200.0, 50.0) + m * uv
    }

    // evaluation.md 3. ワープデフォーマ: 範囲内は双線形
    #[test]
    fn warp_inside_is_bilinear() {
        // 右下の制御点だけ動かした格子
        let mut points = grid(|u, v| Vec2::new(u * 100.0, v * 100.0));
        points[8] = Vec2::new(120.0, 140.0);
        let warp = Warp::new(&points, 2, 2);
        assert_close(warp.map(Vec2::new(0.25, 0.25)), Vec2::new(25.0, 25.0));
        // 右下のセル (50,50)-(100,50)-(50,100)-(120,140) の中心
        assert_close(warp.map(Vec2::new(0.75, 0.75)), Vec2::new(80.0, 85.0));
        // 格子点そのもの
        assert_close(warp.map(Vec2::new(1.0, 1.0)), Vec2::new(120.0, 140.0));
        assert_close(warp.map(Vec2::new(0.5, 0.5)), Vec2::new(50.0, 50.0));
    }

    // evaluation.md 3. ワープデフォーマ: 範囲外 (外挿方式の比較 1)
    // 格子がアフィン (回転・せん断) なら、どちらの方式も格子から遠い点で正確なアフィン写像になる。
    #[test]
    fn warp_outside_affine_grid_matches_both_methods() {
        let points = grid(|u, v| affine(Vec2::new(u, v)));
        let warp = Warp::new(&points, 2, 2);
        for uv in [
            Vec2::new(-3.0, -2.0),
            Vec2::new(4.0, 0.5),
            Vec2::new(0.3, 5.0),
            Vec2::new(-1.5, 2.5),
            Vec2::new(10.0, -10.0),
        ] {
            assert_close(warp.map(uv), affine(uv));
            assert_close(warp.map_edge_bilinear(uv), affine(uv));
        }
    }

    // evaluation.md 3. ワープデフォーマ: 範囲外 (外挿方式の比較 2)
    // 下へ行くほど狭まる台形。端のセルの延長では、格子の外で幅が負になり像が裏返る。
    // 採用した方式 (クランプ + 四隅のアフィン近似) では境界の幅がそのまま続き、裏返らない。
    #[test]
    fn warp_outside_keystone_does_not_fold() {
        let points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(40.0, 100.0),
            Vec2::new(60.0, 100.0),
        ];
        let warp = Warp::new(&points, 1, 1);
        let width =
            |f: &dyn Fn(Vec2) -> Vec2, v: f32| f(Vec2::new(1.0, v)).x - f(Vec2::new(0.0, v)).x;
        let edge = |uv| warp.map_edge_bilinear(uv);
        let chosen = |uv| warp.map(uv);
        // 端のセルの延長: 100 + (20 - 100) * 2 = -60 (裏返る)
        assert!((width(&edge, 2.0) + 60.0).abs() < TOL);
        // 採用した方式: 境界 (v = 1) の幅 20 のまま
        assert!((width(&chosen, 2.0) - 20.0).abs() < TOL);
        assert_close(chosen(Vec2::new(0.5, 2.0)), Vec2::new(50.0, 200.0));
    }

    // evaluation.md 3. ワープデフォーマ: 範囲外 (外挿方式の比較 3)
    // 端のセルだけが局所的に潰れていると、端のセルの延長はそれを格子の外で増幅する。
    #[test]
    fn warp_outside_ignores_local_edge_cell_distortion() {
        // 右端の列を x = 100 から x = 55 へ寄せ、右端のセル幅を 5 にする。
        let points = grid(|u, v| Vec2::new(if u == 1.0 { 55.0 } else { u * 100.0 }, v * 100.0));
        let warp = Warp::new(&points, 2, 2);
        let uv = Vec2::new(2.0, 0.5);
        // 端のセルの延長: 55 + 5 * (4 - 2) = 65 (格子 1 枚分外へ出ても 10px しか進まない)
        assert_close(warp.map_edge_bilinear(uv), Vec2::new(65.0, 50.0));
        // 採用した方式: 四隅の幅 55 で進む
        assert_close(warp.map(uv), Vec2::new(110.0, 50.0));
    }

    // 採用した方式は境界で連続。
    #[test]
    fn warp_outside_is_continuous_at_border() {
        let mut points = grid(|u, v| affine(Vec2::new(u, v)));
        points[2] += Vec2::new(13.0, -7.0);
        points[6] += Vec2::new(-5.0, 9.0);
        let warp = Warp::new(&points, 2, 2);
        for uv in [
            Vec2::new(0.0, 0.3),
            Vec2::new(1.0, 0.7),
            Vec2::new(0.4, 0.0),
            Vec2::new(0.6, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 0.0),
        ] {
            let outward = (uv - Vec2::splat(0.5)).signum() * 1e-3;
            let a = warp.map(uv);
            let b = warp.map(uv + outward);
            assert!(a.distance(b) < 0.5, "{uv}: {a} vs {b}");
        }
    }

    // ワープのヤコビ行列は、セルの境目と格子の境界を避けた点で中心差分と一致する (内側・外側とも)。
    #[test]
    fn warp_jacobian_matches_finite_difference() {
        let mut points = grid(|u, v| affine(Vec2::new(u, v)));
        points[2] += Vec2::new(13.0, -7.0);
        points[4] += Vec2::new(4.0, 6.0);
        points[6] += Vec2::new(-5.0, 9.0);
        let warp = Warp::new(&points, 2, 2);
        let h = 1e-3;
        for uv in [
            Vec2::new(0.2, 0.3),
            Vec2::new(0.7, 0.9),
            Vec2::new(0.35, 0.6),
            Vec2::new(-0.4, 0.3),
            Vec2::new(0.8, 1.7),
            Vec2::new(1.3, -0.6),
            Vec2::new(-2.0, 3.0),
        ] {
            let j = warp.jacobian(uv);
            let du =
                (warp.map(uv + Vec2::new(h, 0.0)) - warp.map(uv - Vec2::new(h, 0.0))) / (2.0 * h);
            let dv =
                (warp.map(uv + Vec2::new(0.0, h)) - warp.map(uv - Vec2::new(0.0, h))) / (2.0 * h);
            assert!(j.x_axis.abs_diff_eq(du, 0.05), "{uv}: {} vs {du}", j.x_axis);
            assert!(j.y_axis.abs_diff_eq(dv, 0.05), "{uv}: {} vs {dv}", j.y_axis);
        }
        // 外側の、はみ出した方向の列はアフィン近似の軸。
        let j = warp.jacobian(Vec2::new(-2.0, 3.0));
        assert_close(j.x_axis, warp.axis_u);
        assert_close(j.y_axis, warp.axis_v);
    }

    // evaluation.md 3. 回転デフォーマ: L(p) = origin + R(angle) * S * p
    #[test]
    fn rotation_local_is_clockwise_in_y_down() {
        let r = Rotation {
            origin: Vec2::new(10.0, 20.0),
            angle: 90.0,
            scale: 2.0,
            reflect_x: false,
            reflect_y: false,
        };
        // (1, 0) (右) は時計回りに 90 度で (0, 1) (下) を向く
        assert_close(
            r.local().transform_point2(Vec2::new(1.0, 0.0)),
            Vec2::new(10.0, 22.0),
        );
        let flipped = Rotation {
            reflect_x: true,
            ..r
        };
        assert_close(
            flipped.local().transform_point2(Vec2::new(1.0, 0.0)),
            Vec2::new(10.0, 18.0),
        );
    }

    // evaluation.md 3. 回転デフォーマ (親がワープ): 原点と向きだけを変換する
    #[test]
    fn rotation_under_warp_uses_origin_and_angle_only() {
        // 90 度回転し 3 倍に拡大した格子
        let points = [
            Vec2::new(300.0, 0.0),
            Vec2::new(300.0, 300.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 300.0),
        ];
        let warp = Warp::new(&points, 1, 1);
        let r = Rotation {
            origin: Vec2::new(0.5, 0.5),
            angle: 0.0,
            scale: 1.0,
            reflect_x: false,
            reflect_y: false,
        };
        let f = Transform::Warp(warp).rotation_child(&r);
        assert_close(f.transform_point2(Vec2::ZERO), Vec2::new(150.0, 150.0));
        // 伸縮は伝わらず、向きだけ 90 度回る
        assert_close(
            f.transform_point2(Vec2::new(10.0, 0.0)),
            Vec2::new(150.0, 160.0),
        );
    }
}
