//! 格子の軸と、セルの座標・インデックスの変換 (`docs/spec/keyform-inference.md#記法`)。

use ugoe_model::{KeyformBinding, KeyformGrid, KeyformOrigin, Parameter};

use crate::grid_index::{cell_coords, key_position};
use crate::{InferError, InferReport, InferScope, KeyformValues};

/// 格子の 1 本の軸。
pub(crate) struct Axis<'a> {
    pub parameter: &'a Parameter,
    pub keys: &'a [f32],
    /// 中立のキーの位置 `z_k`。
    pub neutral: usize,
    /// 線形インデックスでの 1 キーあたりの間隔。
    pub stride: usize,
}

/// 格子の軸と、セルの座標・インデックスの変換。
pub(crate) struct Layout<'a> {
    pub axes: Vec<Axis<'a>>,
    /// セルの数。
    pub len: usize,
}

impl<'a> Layout<'a> {
    /// 格子の軸を `parameters` から引いて作る。
    /// 軸の数・パラメータ・軸の重複・キーの並び (有限で狭義の昇順)・中立のキー・セルの数・
    /// `keyforms` と `origins` の長さを検査する。`ugoe_model::validate` の前のモデルでも、
    /// 誤った値や NaN を作らずにエラーにする。
    pub fn new<T>(
        parameters: &'a [Parameter],
        grid: &'a KeyformGrid<T>,
    ) -> Result<Self, InferError> {
        if grid.bindings.len() > 64 {
            return Err(InferError::TooManyAxes {
                count: grid.bindings.len(),
            });
        }
        // 積があふれたら `None` (`KeyformGrid::expected_len` は検査せずに掛ける)。
        let mut stride = Some(1usize);
        let mut axes: Vec<Axis<'a>> = Vec::with_capacity(grid.bindings.len());
        for binding in &grid.bindings {
            let parameter = parameters
                .iter()
                .find(|p| p.id == binding.parameter)
                .ok_or_else(|| InferError::UnknownParameter {
                    parameter: binding.parameter.clone(),
                })?;
            if axes.iter().any(|a| a.parameter.id == binding.parameter) {
                return Err(InferError::DuplicateAxis {
                    parameter: binding.parameter.clone(),
                });
            }
            check_keys(binding)?;
            let neutral = key_position(&binding.keys, parameter.default).ok_or_else(|| {
                InferError::MissingNeutralKey {
                    parameter: binding.parameter.clone(),
                    default: parameter.default,
                }
            })?;
            axes.push(Axis {
                parameter,
                keys: &binding.keys,
                neutral,
                stride: stride.unwrap_or(usize::MAX),
            });
            stride = stride.and_then(|s| s.checked_mul(binding.keys.len()));
        }

        let len = stride.ok_or(InferError::GridTooLarge)?;
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
        Ok(Self { axes, len })
    }

    /// セル `index` の、軸ごとのキーの位置。
    pub fn coords(&self, index: usize) -> Vec<usize> {
        cell_coords(self.axes.iter().map(|a| a.keys.len()), index).collect()
    }

    /// セル `index` の姿勢: 軸ごとのパラメータの ID と、そのセルでのキーの値。
    pub fn pose(&self, index: usize) -> Vec<(&'a str, f32)> {
        self.axes
            .iter()
            .zip(self.coords(index))
            .map(|(a, i)| (a.parameter.id.as_str(), a.keys[i]))
            .collect()
    }

    /// 中立でない軸の集合 `S(c)` (ビット `k` が軸 `k`)。
    pub fn non_neutral(&self, coords: &[usize]) -> u64 {
        self.axes
            .iter()
            .zip(coords)
            .enumerate()
            .filter(|(_, (a, i))| **i != a.neutral)
            .fold(0, |mask, (k, _)| mask | (1 << k))
    }

    /// `cell(S, c)`: 集合 `mask` の軸だけを `coords` の値にし、残りを中立にしたセル。
    pub fn cell(&self, mask: u64, coords: &[usize]) -> usize {
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

    /// 基準のセル `B` (すべての軸が中立のセル)。
    pub fn base(&self) -> usize {
        self.axes.iter().map(|a| a.neutral * a.stride).sum()
    }

    /// 軸 `k` の、パラメータの値 (セルの座標でのキー) と `default` の差。
    pub fn offset(&self, k: usize, coords: &[usize]) -> f32 {
        let axis = &self.axes[k];
        axis.keys[coords[k]] - axis.parameter.default
    }
}

/// 軸のキーが有限で、狭義の昇順 (重複がない) であることを検査する。1 軸の補間 (`additive.rs` の `line`) は
/// 添字の順を値の順とみなし、重複したキーでは区間の幅が 0 になる。
fn check_keys(binding: &KeyformBinding) -> Result<(), InferError> {
    if let Some(&key) = binding.keys.iter().find(|k| !k.is_finite()) {
        return Err(InferError::NonFiniteKey {
            parameter: binding.parameter.clone(),
            key,
        });
    }
    if binding.keys.windows(2).any(|w| w[0] >= w[1]) {
        return Err(InferError::KeysNotAscending {
            parameter: binding.parameter.clone(),
            keys: binding.keys.clone(),
        });
    }
    Ok(())
}

/// セル `index` を、方法 `method` (由来 `additive` / `depth_rotation` / `mirror`) の推定で
/// 書き換えるか (`docs/spec/keyform-inference.md#セルの由来`)。`authored` のセルは書き換えない。
pub(crate) fn rewrites<T>(
    grid: &KeyformGrid<T>,
    index: usize,
    method: KeyformOrigin,
    scope: InferScope,
) -> bool {
    match scope {
        InferScope::Own => grid.derivation(index) == Some(method),
        InferScope::AllDerived => grid.origin(index).is_derived(),
    }
}

/// セルごとに、推定の元にする (書き換えない) かどうか。`authored` のセルと、`scope` で書き換えない
/// 別の方法の `derived` のセルが元になる。
pub(crate) fn source_cells<T>(
    grid: &KeyformGrid<T>,
    layout: &Layout<'_>,
    method: KeyformOrigin,
    scope: InferScope,
) -> Vec<bool> {
    (0..layout.len)
        .map(|i| !rewrites(grid, i, method, scope))
        .collect()
}

/// 推定の元にする格子を検査する: 基準のセルが元のセルで、元のセルの形が基準と揃っていること。
pub(crate) fn check_sources<T: KeyformValues>(
    grid: &KeyformGrid<T>,
    sources: &[bool],
    base: usize,
) -> Result<(), InferError> {
    if !sources[base] {
        return Err(InferError::BaseNotAuthored { cell: base });
    }
    let shape = grid.keyforms[base].shape_len();
    for (cell, kf) in grid.keyforms.iter().enumerate() {
        if sources[cell] && kf.shape_len() != shape {
            return Err(InferError::ShapeMismatch {
                cell,
                expected: shape,
                found: kf.shape_len(),
            });
        }
    }
    Ok(())
}

/// 作ったセルを格子に書き込み、由来を推定の方法 `method` にする。
pub(crate) fn apply<T>(
    grid: &mut KeyformGrid<T>,
    mut writes: Vec<(usize, T)>,
    method: KeyformOrigin,
) -> InferReport {
    if grid.origins.is_empty() && !writes.is_empty() {
        grid.origins = vec![KeyformOrigin::Authored; grid.keyforms.len()];
    }
    writes.sort_unstable_by_key(|&(cell, _)| cell);
    let mut report = InferReport::default();
    for (cell, kf) in writes {
        grid.keyforms[cell] = kf;
        grid.origins[cell] = method;
        report.written.push(cell);
    }
    report
}

pub(crate) fn flatten<T: KeyformValues>(kf: &T) -> Vec<f32> {
    let mut out = Vec::new();
    kf.write_values(&mut out);
    out
}

/// `base` の複製に値 `v` を書き戻し、クランプしたキーフォーム。
pub(crate) fn build<T: KeyformValues>(base: &T, v: &[f32]) -> T {
    let mut kf = base.clone();
    kf.read_values(v);
    kf.clamp();
    kf
}
