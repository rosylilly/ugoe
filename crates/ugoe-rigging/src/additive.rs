//! 加算合成 (`docs/spec/keyform-inference.md#加算合成`)。

use ugoe_model::{KeyformGrid, KeyformOrigin, Model, Parameter};

use crate::grid::{Axis, Layout, apply, build, check_sources, flatten, source_cells};
use crate::{InferError, KeyformValues};

/// 推定の結果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InferReport {
    /// 書き込んだ (`derived` の) セルの線形インデックス。昇順。
    pub written: Vec<usize>,
}

/// 推定で書き換えるセルの範囲 (`docs/spec/keyform-inference.md#セルの由来`)。
///
/// どちらでも `authored` のセルは書き換えない。書き換えないセルは推定の元にする。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InferScope {
    /// その方法で作ったセルだけを書き換える。方法を記録していない `derived` のセルは、格子の
    /// `inference` の方法ならその方法のセルとみなす。別の方法で作ったセルは書き換えない。
    /// エディタの自動の作り直しが使う。
    Own,
    /// `authored` 以外のすべてのセルを書き換える (明示的に実行した推定)。
    #[default]
    AllDerived,
}

/// 格子の `derived` のセルを加算合成で作り直す (`docs/spec/keyform-inference.md#加算合成`)。
///
/// 軸のパラメータは `model` から引く。書き換えるのは `scope` のセルで、書いたセルの由来は
/// `additive` になる。`authored` のセルは変えない。
/// 格子の `inference` の設定は `scope` の判定にだけ使う (どの方法で作り直すかは呼び出し側が決める)。
/// エラーのときは格子を変えない。
pub fn infer_additive<T: KeyformValues>(
    model: &Model,
    grid: &mut KeyformGrid<T>,
    scope: InferScope,
) -> Result<InferReport, InferError> {
    infer_additive_with(&model.parameters, grid, scope)
}

pub(crate) fn infer_additive_with<T: KeyformValues>(
    parameters: &[Parameter],
    grid: &mut KeyformGrid<T>,
    scope: InferScope,
) -> Result<InferReport, InferError> {
    let layout = Layout::new(parameters, grid)?;
    let authored = source_cells(grid, &layout, KeyformOrigin::Additive, scope);
    let base = layout.base();
    check_sources(grid, &authored, base)?;

    let base_kf = &grid.keyforms[base];
    let values = (0..layout.len)
        .map(|i| authored[i].then(|| flatten(&grid.keyforms[i])))
        .collect();
    let written = combine(&layout, &authored, values, &flatten(base_kf), |_, v| {
        flatten(&build(base_kf, v))
    });
    let writes = written
        .into_iter()
        .map(|(cell, v)| (cell, build(base_kf, &v)))
        .collect();
    Ok(apply(grid, writes, KeyformOrigin::Additive))
}

/// 加算合成の本体。値の列 (`KeyformValues::write_values` の並び) の上で計算する。
///
/// `authored` は推定の元にする (書き換えない) セルで、`values` はそのセルだけが `Some`。`finish` は推定した生の値を、セルに書く値に直す
/// (クランプなど)。1 軸セルは直した値を多軸セルの `Δ_k` に使う。
/// 戻り値は作った (`derived` の) セルと、その直した値。
pub(crate) fn combine(
    layout: &Layout<'_>,
    authored: &[bool],
    mut values: Vec<Option<Vec<f32>>>,
    base_values: &[f32],
    mut finish: impl FnMut(usize, &[f32]) -> Vec<f32>,
) -> Vec<(usize, Vec<f32>)> {
    let len = layout.len;
    let base = layout.base();
    let mut written = Vec::new();

    // 1 軸セル: 同じ軸上の `authored` のセルから補間・外挿する。
    for axis in &layout.axes {
        // 軸上のキー `i` の 1 軸セル (`i == z_k` なら基準のセル)。
        let on_line = |i: usize| base - axis.neutral * axis.stride + i * axis.stride;
        let sources: Vec<usize> = (0..axis.keys.len())
            .filter(|&i| authored[on_line(i)])
            .collect();
        for i in 0..axis.keys.len() {
            let cell = on_line(i);
            if authored[cell] {
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
                        _ => base_values.to_vec(),
                    }
                }
            };
            let v = finish(cell, &v);
            written.push((cell, v.clone()));
            values[cell] = Some(v);
        }
    }

    // `Δ_k(c)` は 1 軸セルの値から基準の値を引いたもの。
    let delta = |cell: usize, out: &mut [f32], sign: f32| {
        if let Some(v) = &values[cell] {
            for ((o, x), b) in out.iter_mut().zip(v).zip(base_values) {
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
        if !authored[cell] {
            continue;
        }
        let coords = layout.coords(cell);
        let set = layout.non_neutral(&coords);
        let mut r = values[cell].clone().unwrap_or_default();
        for (o, b) in r.iter_mut().zip(base_values) {
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
        if authored[cell] {
            continue;
        }
        let coords = layout.coords(cell);
        let set = layout.non_neutral(&coords);
        let mut v = base_values.to_vec();
        for_each_axis(set, |k| delta(layout.cell(1 << k, &coords), &mut v, 1.0));
        for_each_proper_subset(set, |sub| {
            if let Some(term) = &interaction[layout.cell(sub, &coords)] {
                for (o, t) in v.iter_mut().zip(term) {
                    *o += t;
                }
            }
        });
        written.push((cell, finish(cell, &v)));
    }
    written
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
