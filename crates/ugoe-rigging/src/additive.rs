//! 加算合成 (`docs/spec/keyform-inference.md#加算合成`)。

use ugoe_model::{KeyformGrid, KeyformOrigin, Model, Parameter};

use crate::{InferError, KeyformValues};

/// 中立のキーとパラメータの `default` を等しいとみなす誤差。
const NEUTRAL_EPSILON: f32 = 1e-5;

/// 推定の結果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InferReport {
    /// 書き込んだ (`derived` の) セルの線形インデックス。昇順。
    pub written: Vec<usize>,
}

/// 格子の `derived` のセルを加算合成で作り直す (`docs/spec/keyform-inference.md#加算合成`)。
///
/// 軸のパラメータは `model` から引く。`authored` のセルは変えない。
/// 格子の `inference` の設定は見ない (どの方法で作り直すかは呼び出し側が決める)。
/// エラーのときは格子を変えない。
pub fn infer_additive<T: KeyformValues>(
    model: &Model,
    grid: &mut KeyformGrid<T>,
) -> Result<InferReport, InferError> {
    infer_additive_with(&model.parameters, grid)
}

/// 格子の 1 本の軸。
struct Axis<'a> {
    keys: &'a [f32],
    /// 中立のキーの位置 `z_k`。
    neutral: usize,
    /// 線形インデックスでの 1 キーあたりの間隔。
    stride: usize,
}

/// 格子の軸と、セルの座標・インデックスの変換。
struct Layout<'a> {
    axes: Vec<Axis<'a>>,
}

impl Layout<'_> {
    fn coords(&self, mut index: usize) -> Vec<usize> {
        self.axes
            .iter()
            .map(|a| {
                let i = index % a.keys.len();
                index /= a.keys.len();
                i
            })
            .collect()
    }

    /// 中立でない軸の集合 `S(c)` (ビット `k` が軸 `k`)。
    fn non_neutral(&self, coords: &[usize]) -> u64 {
        self.axes
            .iter()
            .zip(coords)
            .enumerate()
            .filter(|(_, (a, i))| **i != a.neutral)
            .fold(0, |mask, (k, _)| mask | (1 << k))
    }

    /// `cell(S, c)`: 集合 `mask` の軸だけを `coords` の値にし、残りを中立にしたセル。
    fn cell(&self, mask: u64, coords: &[usize]) -> usize {
        self.axes
            .iter()
            .zip(coords)
            .enumerate()
            .map(|(k, (a, &i))| {
                let i = if mask & (1 << k) != 0 { i } else { a.neutral };
                i * a.stride
            })
            .sum()
    }

    fn base(&self) -> usize {
        self.axes.iter().map(|a| a.neutral * a.stride).sum()
    }
}

pub(crate) fn infer_additive_with<T: KeyformValues>(
    parameters: &[Parameter],
    grid: &mut KeyformGrid<T>,
) -> Result<InferReport, InferError> {
    if grid.bindings.len() > 64 {
        return Err(InferError::TooManyAxes {
            count: grid.bindings.len(),
        });
    }
    let mut stride = 1;
    let mut axes = Vec::with_capacity(grid.bindings.len());
    for binding in &grid.bindings {
        let parameter = parameters
            .iter()
            .find(|p| p.id == binding.parameter)
            .ok_or_else(|| InferError::UnknownParameter {
                parameter: binding.parameter.clone(),
            })?;
        let neutral = binding
            .keys
            .iter()
            .position(|&k| (k - parameter.default).abs() <= NEUTRAL_EPSILON)
            .ok_or_else(|| InferError::MissingNeutralKey {
                parameter: binding.parameter.clone(),
                default: parameter.default,
            })?;
        axes.push(Axis {
            keys: &binding.keys,
            neutral,
            stride,
        });
        stride *= binding.keys.len();
    }
    let layout = Layout { axes };

    let len = grid.expected_len();
    if grid.keyforms.len() != len {
        return Err(InferError::GridSizeMismatch {
            expected: len,
            found: grid.keyforms.len(),
        });
    }
    if !grid.origins.is_empty() && grid.origins.len() != len {
        return Err(InferError::OriginsLengthMismatch {
            expected: len,
            found: grid.origins.len(),
        });
    }
    let authored = |i: usize| grid.origin(i) == KeyformOrigin::Authored;

    let base = layout.base();
    if !authored(base) {
        return Err(InferError::BaseNotAuthored { cell: base });
    }
    let base_kf = &grid.keyforms[base];
    let shape = base_kf.shape_len();
    for (cell, kf) in grid.keyforms.iter().enumerate() {
        if authored(cell) && kf.shape_len() != shape {
            return Err(InferError::ShapeMismatch {
                cell,
                expected: shape,
                found: kf.shape_len(),
            });
        }
    }

    // セルの値の列。`authored` のセルと、作った 1 軸セルだけが入る。
    let mut values: Vec<Option<Vec<f32>>> = (0..len)
        .map(|i| authored(i).then(|| flatten(&grid.keyforms[i])))
        .collect();
    let base_values = flatten(base_kf);
    // (線形インデックス, 作ったキーフォーム)
    let mut writes: Vec<(usize, T)> = Vec::new();
    let mut make = |cell: usize, v: &[f32]| {
        let mut kf = base_kf.clone();
        kf.read_values(v);
        kf.clamp();
        let clamped = flatten(&kf);
        writes.push((cell, kf));
        clamped
    };

    // 1 軸セル: 同じ軸上の `authored` のセルから補間・外挿する。
    for axis in &layout.axes {
        // 軸上のキー `i` の 1 軸セル (`i == z_k` なら基準のセル)。
        let on_line = |i: usize| base - axis.neutral * axis.stride + i * axis.stride;
        let sources: Vec<usize> = (0..axis.keys.len())
            .filter(|&i| authored(on_line(i)))
            .collect();
        for i in 0..axis.keys.len() {
            let cell = on_line(i);
            if authored(cell) {
                continue;
            }
            let below = sources.iter().rev().copied().find(|&j| j < i);
            let above = sources.iter().copied().find(|&j| j > i);
            let v = match (below, above) {
                (Some(a), Some(b)) => line(axis, &values, on_line, a, b, i),
                _ => {
                    // 片側だけ: その側の最も近い 2 つ (基準のセルは必ず含まれる)。
                    let mut side: Vec<usize> = if below.is_some() {
                        sources.iter().rev().copied().filter(|&j| j < i).collect()
                    } else {
                        sources.iter().copied().filter(|&j| j > i).collect()
                    };
                    side.truncate(2);
                    match side[..] {
                        [a, b] => line(axis, &values, on_line, a, b, i),
                        _ => base_values.clone(),
                    }
                }
            };
            values[cell] = Some(make(cell, &v));
        }
    }

    // `Δ_k(c)` は 1 軸セルの値から基準の値を引いたもの。
    let delta = |cell: usize, out: &mut [f32], sign: f32| {
        if let Some(v) = &values[cell] {
            for ((o, x), b) in out.iter_mut().zip(v).zip(&base_values) {
                *o += sign * (x - b);
            }
        }
    };

    // 交互作用の項 `R(T, a)`。軸の少ないセルから順に作る。
    let mut order: Vec<(u32, usize)> = (0..len)
        .map(|i| (layout.non_neutral(&layout.coords(i)).count_ones(), i))
        .filter(|&(n, _)| n >= 2)
        .collect();
    order.sort_unstable();
    let mut interaction: Vec<Option<Vec<f32>>> = vec![None; len];
    for &(_, cell) in &order {
        if !authored(cell) {
            continue;
        }
        let coords = layout.coords(cell);
        let set = layout.non_neutral(&coords);
        let mut r = values[cell].clone().unwrap_or_default();
        for (o, b) in r.iter_mut().zip(&base_values) {
            *o -= b;
        }
        for_each_axis(set, |k| delta(layout.cell(1 << k, &coords), &mut r, -1.0));
        for_each_proper_subset(set, |sub| {
            if let Some(term) = &interaction[layout.cell(sub, &coords)] {
                for (o, t) in r.iter_mut().zip(term) {
                    *o -= t;
                }
            }
        });
        interaction[cell] = Some(r);
    }

    // 多軸セル: `B + Σ Δ_k(c) + Σ R(U, c)`。
    for &(_, cell) in &order {
        if authored(cell) {
            continue;
        }
        let coords = layout.coords(cell);
        let set = layout.non_neutral(&coords);
        let mut v = base_values.clone();
        for_each_axis(set, |k| delta(layout.cell(1 << k, &coords), &mut v, 1.0));
        for_each_proper_subset(set, |sub| {
            if let Some(term) = &interaction[layout.cell(sub, &coords)] {
                for (o, t) in v.iter_mut().zip(term) {
                    *o += t;
                }
            }
        });
        make(cell, &v);
    }

    if grid.origins.is_empty() && !writes.is_empty() {
        grid.origins = vec![KeyformOrigin::Authored; len];
    }
    writes.sort_unstable_by_key(|&(cell, _)| cell);
    let mut report = InferReport::default();
    for (cell, kf) in writes {
        grid.keyforms[cell] = kf;
        grid.origins[cell] = KeyformOrigin::Derived;
        report.written.push(cell);
    }
    Ok(report)
}

fn flatten<T: KeyformValues>(kf: &T) -> Vec<f32> {
    let mut out = Vec::new();
    kf.write_values(&mut out);
    out
}

/// 軸上のキー `a` と `b` のセルを通る直線の、キー `i` での値 (補間・外挿)。
fn line(
    axis: &Axis<'_>,
    values: &[Option<Vec<f32>>],
    on_line: impl Fn(usize) -> usize,
    a: usize,
    b: usize,
    i: usize,
) -> Vec<f32> {
    let (xa, xb, x) = (axis.keys[a], axis.keys[b], axis.keys[i]);
    let t = (x - xa) / (xb - xa);
    let va = values[on_line(a)].as_deref().unwrap_or_default();
    let vb = values[on_line(b)].as_deref().unwrap_or_default();
    va.iter().zip(vb).map(|(p, q)| p + t * (q - p)).collect()
}

fn for_each_axis(set: u64, mut f: impl FnMut(usize)) {
    let mut rest = set;
    while rest != 0 {
        f(rest.trailing_zeros() as usize);
        rest &= rest - 1;
    }
}

/// `set` の真部分集合のうち、要素が 2 つ以上のものを列挙する。
fn for_each_proper_subset(set: u64, mut f: impl FnMut(u64)) {
    let mut sub = set.wrapping_sub(1) & set;
    while sub != 0 {
        if sub.count_ones() >= 2 {
            f(sub);
        }
        sub = (sub - 1) & set;
    }
}
