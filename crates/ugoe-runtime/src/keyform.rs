//! キーフォーム格子の補間 (`docs/spec/evaluation.md#1-キーフォームの重み`)。
//!
//! 補間後の値はどの補間の種類でもキーフォームの線形結合 `Σ w_i keyform[i]` で、ここでは重みを求める。

use alloc::vec::Vec;
use ugoe_model::{KeyformGrid, KeyformInterpolation};

use crate::ParamIndex;

/// ロード時に解決済みのキーフォーム軸。
#[derive(Debug, Clone)]
pub struct ResolvedBinding {
    /// 軸のパラメータ。
    pub param: ParamIndex,
    /// 軸のキー (空でなく、狭義の昇順。モデルの検証で確かめる)。
    pub keys: Vec<f32>,
    /// 曲線補間の軸で、キーごとの接線の係数 (`docs/spec/evaluation.md#曲線補間`)。
    /// キー `i` の接線は、キー `j = tangent_base(i)` から 3 つのキーの値に `tangents[i]` を掛けた和。
    /// 空なら線形 (線形の格子と、キーが 2 つ以下の軸)。
    pub tangents: Vec<[f32; 3]>,
}

impl ResolvedBinding {
    /// 軸を作る。`interpolation` が曲線でキーが 3 つ以上なら、接線の係数を求めておく。
    /// `keys` は空でなく、狭義の昇順であること (空だと重みを求めるときにパニックする)。
    pub fn new(param: ParamIndex, keys: Vec<f32>, interpolation: KeyformInterpolation) -> Self {
        let tangents =
            if interpolation.effective() == KeyformInterpolation::Smooth && keys.len() >= 3 {
                (0..keys.len()).map(|i| tangent(&keys, i)).collect()
            } else {
                Vec::new()
            };
        Self {
            param,
            keys,
            tangents,
        }
    }

    /// 曲線で補間する軸か。
    pub fn is_smooth(&self) -> bool {
        !self.tangents.is_empty()
    }

    /// 軸 1 本の重みの最大の個数 (線形は 2、曲線は 4)。
    pub fn max_weights(&self) -> usize {
        if self.is_smooth() {
            self.keys.len().min(4)
        } else {
            2
        }
    }
}

/// キー `i` の接線に使う 3 つのキーの先頭 (端では内側の 3 つ)。キーは 3 つ以上。
fn tangent_base(n: usize, i: usize) -> usize {
    i.saturating_sub(1).min(n - 3)
}

/// キー `i` での接線 (キーの値に対する微分) を、3 つのキー `j, j+1, j+2` の値の係数で表す。
///
/// 3 つのキーを通る放物線のキー `i` での微分 (内側は Bessel の接線、端は放物線の端の微分)。
/// 係数の和は 0 (値が一定なら接線は 0)。計算は f64 で行い、f32 に丸める。
fn tangent(keys: &[f32], i: usize) -> [f32; 3] {
    let j = tangent_base(keys.len(), i);
    let (x0, x1, x2) = (
        f64::from(keys[j]),
        f64::from(keys[j + 1]),
        f64::from(keys[j + 2]),
    );
    let (h0, h1) = (x1 - x0, x2 - x1);
    let sum = h0 + h1;
    // 接線 = a * d0 + b * d1 (d0, d1 は 2 つの区間の傾き)。
    let (a, b) = match i - j {
        0 => ((2.0 * h0 + h1) / sum, -h0 / sum),
        1 => (h1 / sum, h0 / sum),
        _ => (-h1 / sum, (2.0 * h1 + h0) / sum),
    };
    [(-a / h0) as f32, (a / h0 - b / h1) as f32, (b / h1) as f32]
}

/// 補間に使う (キーフォームの線形インデックス, 重み) の組。重みの総和は 1 (曲線補間では負の重みもある)。
pub type Weights = Vec<(usize, f32)>;

/// 各軸の値からキーフォーム重みを計算する (形のフィールド用)。
///
/// 軸ごとに値をキー範囲にクランプし、線形の軸は隣接 2 キー、曲線の軸は最大 4 キーの重みを求め、
/// 全軸の組み合わせの積を重みとする。重み 0 の項は含めない。
/// `values` はパラメータの現在値 ([`crate::ParameterValues::as_slice`]) で、各軸の `param` を添字に読む。
pub fn weights(bindings: &[ResolvedBinding], values: &[f32], out: &mut Weights) {
    weights_with(bindings, values, true, out);
}

/// すべての軸を線形として重みを計算する (不透明度・描画順・色など形以外のフィールド用)。
/// 曲線の軸がなければ [`weights`] と同じ。
pub fn linear_weights(bindings: &[ResolvedBinding], values: &[f32], out: &mut Weights) {
    weights_with(bindings, values, false, out);
}

fn weights_with(bindings: &[ResolvedBinding], values: &[f32], smooth: bool, out: &mut Weights) {
    out.clear();
    out.push((0, 1.0));
    let mut stride = 1;
    for binding in bindings {
        let (seg, t) = locate(&binding.keys, values[binding.param.0 as usize]);
        let n = out.len();
        if smooth && binding.is_smooth() && t > 0.0 {
            let (base, axis) = smooth_axis(binding, seg, t);
            let count = axis.len().min(binding.keys.len() - base);
            for i in 0..n {
                let (index, w) = out[i];
                out[i] = (index + base * stride, w * axis[0]);
                for (k, a) in axis.iter().enumerate().take(count).skip(1) {
                    out.push((index + (base + k) * stride, w * a));
                }
            }
        } else {
            for i in 0..n {
                let (index, w) = out[i];
                out[i] = (index + seg * stride, w * (1.0 - t));
                if t > 0.0 {
                    out.push((index + (seg + 1) * stride, w * t));
                }
            }
        }
        stride *= binding.keys.len();
    }
    // 曲線補間の重みは負にもなる。NaN は線形と同じく取り除く。
    out.retain(|&(_, w)| w.abs() > 0.0);
}

/// 曲線の軸の、区間 `seg` の位置 `t` (0 < t < 1) での 3 次エルミートの重み。
/// キー `base..base + 4` の重みを返す (範囲外のキーの重みは 0)。
fn smooth_axis(binding: &ResolvedBinding, seg: usize, t: f32) -> (usize, [f32; 4]) {
    let n = binding.keys.len();
    let base = seg.saturating_sub(1);
    let h = binding.keys[seg + 1] - binding.keys[seg];
    let (t2, t3) = (t * t, t * t * t);
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    let mut axis = [0.0f32; 4];
    axis[seg - base] += h00;
    axis[seg + 1 - base] += h01;
    for (key, basis) in [(seg, h10), (seg + 1, h11)] {
        let j = tangent_base(n, key);
        for (c, coef) in binding.tangents[key].iter().enumerate() {
            axis[j + c - base] += h * basis * coef;
        }
    }
    (base, axis)
}

/// 値 `v` が属するセグメントの開始キーインデックスと補間係数 t (0..1)。
fn locate(keys: &[f32], v: f32) -> (usize, f32) {
    debug_assert!(!keys.is_empty());
    let last = keys.len() - 1;
    if last == 0 || v <= keys[0] {
        return (0, 0.0);
    }
    if v >= keys[last] {
        return (last, 0.0);
    }
    // keys[seg] <= v < keys[seg + 1]
    let seg = keys.partition_point(|&k| k <= v) - 1;
    let t = (v - keys[seg]) / (keys[seg + 1] - keys[seg]);
    (seg, t)
}

/// 形のフィールドの重み (`shape`) と、形以外のフィールドの重み (`linear`) の作業領域。
#[derive(Debug, Default)]
pub struct GridWeights {
    pub(crate) shape: Weights,
    pub(crate) linear: Weights,
}

impl GridWeights {
    /// 重みの列を `capacity` 個ずつ確保して作る。
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            shape: Weights::with_capacity(capacity),
            linear: Weights::with_capacity(capacity),
        }
    }

    /// 重みを求めて `(形の重み, 形以外の重み)` を返す。曲線の軸がなければ両方とも同じ列
    /// (線形の格子は 1 回だけ計算する)。
    pub fn compute(
        &mut self,
        bindings: &[ResolvedBinding],
        values: &[f32],
    ) -> (&Weights, &Weights) {
        weights(bindings, values, &mut self.shape);
        if bindings.iter().any(ResolvedBinding::is_smooth) {
            linear_weights(bindings, values, &mut self.linear);
            (&self.shape, &self.linear)
        } else {
            (&self.shape, &self.shape)
        }
    }
}

/// 重みでスカラーを合成する。
pub fn blend_scalar<T>(grid: &KeyformGrid<T>, weights: &Weights, f: impl Fn(&T) -> f32) -> f32 {
    weights.iter().map(|&(i, w)| f(&grid.keyforms[i]) * w).sum()
}

/// 重みでベクトルを合成する (和は重みの順序で取る)。
pub fn blend_vec2<T>(
    grid: &KeyformGrid<T>,
    weights: &Weights,
    f: impl Fn(&T) -> ugoe_core::Vec2,
) -> ugoe_core::Vec2 {
    let mut sum = ugoe_core::Vec2::ZERO;
    for &(i, w) in weights {
        sum += f(&grid.keyforms[i]) * w;
    }
    sum
}

/// 真偽値のフィールドを合成する: 重みが最大の要素の値 (同じ重みなら小さいインデックスを優先)。
pub fn blend_bool<T>(grid: &KeyformGrid<T>, weights: &Weights, f: impl Fn(&T) -> bool) -> bool {
    let mut best: Option<(usize, f32)> = None;
    for &(i, w) in weights {
        if best.is_none_or(|(bi, bw)| w > bw || (w == bw && i < bi)) {
            best = Some((i, w));
        }
    }
    best.is_some_and(|(i, _)| f(&grid.keyforms[i]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ugoe_core::math;

    fn binding(param: u32, keys: &[f32]) -> ResolvedBinding {
        ResolvedBinding::new(
            ParamIndex(param),
            keys.to_vec(),
            KeyformInterpolation::Linear,
        )
    }

    fn smooth(param: u32, keys: &[f32]) -> ResolvedBinding {
        ResolvedBinding::new(
            ParamIndex(param),
            keys.to_vec(),
            KeyformInterpolation::Smooth,
        )
    }

    /// 1 軸のスカラーのキー値 `values` を重みで補間する。
    fn interp(b: &ResolvedBinding, keyvalues: &[f32], v: f32) -> f32 {
        let mut w = Weights::new();
        weights(core::slice::from_ref(b), &[v], &mut w);
        w.iter().map(|&(i, w)| keyvalues[i] * w).sum()
    }

    #[test]
    fn no_bindings_is_single_keyform() {
        let mut w = Weights::new();
        weights(&[], &[], &mut w);
        assert_eq!(w, vec![(0, 1.0)]);
    }

    #[test]
    fn one_axis_interpolates_and_clamps() {
        let b = [binding(0, &[-1.0, 0.0, 1.0])];
        let mut w = Weights::new();
        weights(&b, &[0.5], &mut w);
        assert_eq!(w, vec![(1, 0.5), (2, 0.5)]);
        weights(&b, &[5.0], &mut w);
        assert_eq!(w, vec![(2, 1.0)]);
        weights(&b, &[-5.0], &mut w);
        assert_eq!(w, vec![(0, 1.0)]);
    }

    // evaluation.md 1. 多重線形の重み: 真偽値は重みが最大の要素、同じ重みなら小さいインデックス
    #[test]
    fn bool_uses_max_weight_with_lowest_index_on_tie() {
        let grid = KeyformGrid::new(vec![], vec![false, true, true]);
        assert!(!blend_bool(&grid, &vec![(1, 0.5), (0, 0.5)], |&b| b));
        assert!(blend_bool(&grid, &vec![(0, 0.4), (1, 0.6)], |&b| b));
        assert!(blend_bool(
            &grid,
            &vec![(2, 0.5), (0, 0.25), (1, 0.25)],
            |&b| b
        ));
    }

    #[test]
    fn two_axes_bilinear() {
        // 軸 0: 2 キー, 軸 1: 2 キー → 格子 2x2、線形インデックス i0 + 2*i1
        let b = [binding(0, &[0.0, 1.0]), binding(1, &[0.0, 1.0])];
        let mut w = Weights::new();
        weights(&b, &[0.25, 0.5], &mut w);
        w.sort_by_key(|&(i, _)| i);
        assert_eq!(w, vec![(0, 0.375), (1, 0.125), (2, 0.375), (3, 0.125)]);
    }

    // evaluation.md 曲線補間: キーの上ではキーの値そのもの (重みは 1 つ)
    #[test]
    fn smooth_is_exact_on_keys_and_clamps() {
        let b = [smooth(0, &[-30.0, 0.0, 10.0, 30.0])];
        let mut w = Weights::new();
        for (i, v) in [-30.0, 0.0, 10.0, 30.0].into_iter().enumerate() {
            weights(&b, &[v], &mut w);
            assert_eq!(w, vec![(i, 1.0)]);
        }
        weights(&b, &[-99.0], &mut w);
        assert_eq!(w, vec![(0, 1.0)]);
        weights(&b, &[99.0], &mut w);
        assert_eq!(w, vec![(3, 1.0)]);
    }

    // evaluation.md 曲線補間: キーが 2 つの軸は線形とビット単位で同じ
    #[test]
    fn smooth_with_two_keys_is_linear() {
        let (s, l) = (smooth(0, &[-1.0, 1.0]), binding(0, &[-1.0, 1.0]));
        assert!(!s.is_smooth());
        let (mut ws, mut wl) = (Weights::new(), Weights::new());
        for v in [-1.0, -0.37, 0.0, 0.5, 0.999] {
            weights(core::slice::from_ref(&s), &[v], &mut ws);
            weights(core::slice::from_ref(&l), &[v], &mut wl);
            assert_eq!(ws, wl);
        }
    }

    // evaluation.md 曲線補間: 重みの和は 1、3 つのキーなら放物線を正確に再現する
    #[test]
    fn smooth_reproduces_parabolas_and_sums_to_one() {
        let keys = [-30.0, -10.0, 0.0, 25.0];
        let b = smooth(0, &keys);
        let f = |x: f32| 0.5 * x * x - 3.0 * x + 7.0;
        let values: Vec<f32> = keys.iter().map(|&k| f(k)).collect();
        let mut w = Weights::new();
        let mut v = -30.0;
        while v <= 25.0 {
            weights(core::slice::from_ref(&b), &[v], &mut w);
            let sum: f32 = w.iter().map(|&(_, w)| w).sum();
            assert!((sum - 1.0).abs() < 1e-5, "sum {sum} at {v}");
            let got = interp(&b, &values, v);
            assert!((got - f(v)).abs() < 1e-2, "{got} != {} at {v}", f(v));
            v += 0.5;
        }
    }

    // evaluation.md 曲線補間: 3 つのキーが円弧上にあるとき、中間の値は直線より円弧に近い
    #[test]
    fn smooth_follows_an_arc_better_than_linear() {
        let keys = [-30.0f32, 0.0, 30.0];
        let (xs, ys): (Vec<f32>, Vec<f32>) =
            keys.iter().map(|k| math::sin_cos(k.to_radians())).unzip();
        let (s, l) = (smooth(0, &keys), binding(0, &keys));
        for v in [-25.0f32, -15.0, -5.0, 5.0, 15.0, 25.0] {
            let radius = |b: &ResolvedBinding| math::hypot(interp(b, &xs, v), interp(b, &ys, v));
            let (smooth_err, linear_err) = ((radius(&s) - 1.0).abs(), (radius(&l) - 1.0).abs());
            assert!(
                smooth_err * 5.0 < linear_err,
                "{smooth_err} vs {linear_err} at {v}"
            );
        }
    }

    // evaluation.md 曲線補間: 軸ごとに線形と曲線を混ぜられる。形以外のフィールドは線形の重み
    #[test]
    fn mixed_axes_and_linear_weights() {
        let b = [smooth(0, &[0.0, 1.0, 2.0]), binding(1, &[0.0, 1.0])];
        let mut w = Weights::new();
        weights(&b, &[0.5, 0.25], &mut w);
        // 曲線の軸で 3 キー × 線形の軸で 2 キー。
        assert_eq!(w.len(), 6);
        let sum: f32 = w.iter().map(|&(_, w)| w).sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert!(w.iter().any(|&(_, w)| w < 0.0));
        let mut lin = Weights::new();
        linear_weights(&b, &[0.5, 0.25], &mut lin);
        let mut reference = Weights::new();
        let plain = [binding(0, &[0.0, 1.0, 2.0]), binding(1, &[0.0, 1.0])];
        weights(&plain, &[0.5, 0.25], &mut reference);
        assert_eq!(lin, reference);
        let mut gw = GridWeights::default();
        let (shape, linear) = gw.compute(&b, &[0.5, 0.25]);
        assert_eq!((shape, linear), (&w, &reference));
    }
}
