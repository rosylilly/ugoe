//! キーフォーム格子の多重線形補間。

use ugoe_model::KeyformGrid;

use crate::ParamIndex;

/// ロード時に解決済みのキーフォーム軸。
#[derive(Debug, Clone)]
pub struct ResolvedBinding {
    pub param: ParamIndex,
    pub keys: Vec<f32>,
}

/// 補間に使う (キーフォームの線形インデックス, 重み) の組。重みの総和は 1。
pub type Weights = Vec<(usize, f32)>;

/// 各軸の値からキーフォーム重みを計算する。
///
/// 軸ごとに値をキー範囲にクランプし、隣接 2 キー間の線形補間係数を求め、
/// 全軸の組み合わせ (最大 2^N 通り) の積を重みとする。重み 0 の項は含めない。
pub fn weights(bindings: &[ResolvedBinding], values: &[f32], out: &mut Weights) {
    out.clear();
    out.push((0, 1.0));
    let mut stride = 1;
    for binding in bindings {
        let (seg, t) = locate(&binding.keys, values[binding.param.0 as usize]);
        let n = out.len();
        for i in 0..n {
            let (index, w) = out[i];
            out[i] = (index + seg * stride, w * (1.0 - t));
            if t > 0.0 {
                out.push((index + (seg + 1) * stride, w * t));
            }
        }
        stride *= binding.keys.len();
    }
    out.retain(|&(_, w)| w > 0.0);
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

    fn binding(param: u32, keys: &[f32]) -> ResolvedBinding {
        ResolvedBinding {
            param: ParamIndex(param),
            keys: keys.to_vec(),
        }
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
        let grid = KeyformGrid {
            bindings: vec![],
            keyforms: vec![false, true, true],
        };
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
}
