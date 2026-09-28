//! キーフォーム格子の編集の補助。格子の並びは `ugoe_model::KeyformGrid` の doc コメントを参照。

use ugoe_model::{KeyformBinding, KeyformGrid, KeyformInference, KeyformOrigin, ParamId};
use ugoe_rigging::KeyformValues;

use crate::command::KeyformCell;

/// パラメータ `parameter` の軸を格子から取り除き、キー `value` の断面だけを残す。
///
/// `value` と等しいキーがなければ最も近いキー (等距離なら小さい方) の断面を残す。
/// 深度からの回転の `yaw` / `pitch` がこのパラメータなら外す (回転の中心などの設定は残す)。
/// 軸がなければ何もしない。
pub(crate) fn collapse_axis<T>(grid: &mut KeyformGrid<T>, parameter: &ParamId, value: f32) {
    if let Some(KeyformInference::DepthRotation(rotation)) = &mut grid.inference {
        for axis in [&mut rotation.yaw, &mut rotation.pitch] {
            if axis.as_ref() == Some(parameter) {
                *axis = None;
            }
        }
    }
    let Some(axis) = grid.bindings.iter().position(|b| &b.parameter == parameter) else {
        return;
    };
    let keys = &grid.bindings[axis].keys;
    let Some(key) = nearest_key(keys, value) else {
        // キーのない軸 (不正な格子) は、軸だけを取り除く。
        grid.bindings.remove(axis);
        return;
    };
    let keep = slice_mask(grid, axis, key);
    retain_cells(&mut grid.keyforms, &keep);
    // `origins` は空 (すべて `authored`) か、キーフォームと同じ並び。
    if !grid.origins.is_empty() {
        retain_cells(&mut grid.origins, &keep);
    }
    grid.bindings.remove(axis);
}

/// `value` と等しいキー、なければ最も近いキーのインデックス。等距離なら先 (小さい方) を採る。
fn nearest_key(keys: &[f32], value: f32) -> Option<usize> {
    keys.iter().position(|&k| k == value).or_else(|| {
        keys.iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (*a - value).abs().total_cmp(&(*b - value).abs()))
            .map(|(i, _)| i)
    })
}

/// 軸 `axis` のインデックスが `key` であるセルを `true` にしたマスク (長さはキーフォーム数)。
fn slice_mask<T>(grid: &KeyformGrid<T>, axis: usize, key: usize) -> Vec<bool> {
    let stride: usize = grid.bindings[..axis]
        .iter()
        .map(|b| b.keys.len().max(1))
        .product();
    let len = grid.bindings[axis].keys.len();
    (0..grid.keyforms.len())
        .map(|i| (i / stride) % len == key)
        .collect()
}

fn retain_cells<T>(cells: &mut Vec<T>, keep: &[bool]) {
    let mut index = 0;
    cells.retain(|_| {
        let kept = keep.get(index).copied().unwrap_or(false);
        index += 1;
        kept
    });
}

/// セルの指定が不正な理由。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CellError {
    #[error("axis `{0}` is not specified")]
    MissingAxis(String),
    #[error("parameter `{0}` is not an axis of the grid")]
    UnknownAxis(String),
    #[error("{value} is not a key of axis `{parameter}`")]
    UnknownKey { parameter: String, value: f32 },
    #[error("cell index {index} out of range (keyforms = {len})")]
    OutOfRange { index: usize, len: usize },
    #[error("{value} is already a key of axis `{parameter}`")]
    DuplicateKey { parameter: String, value: f32 },
    #[error("key {value} of axis `{parameter}` is not finite")]
    NonFiniteKey { parameter: String, value: f32 },
    /// 格子のキーフォーム数が軸のキー数の積と合わない (不正な格子)。
    #[error("grid has {actual} keyforms but its axes need {expected}")]
    KeyformCount { expected: usize, actual: usize },
    /// 新しいキーフォームの頂点 (制御点) の数が、置き換えるセルと違う。
    #[error("keyform has {actual} vertices but the cell has {expected}")]
    ShapeMismatch { expected: usize, actual: usize },
}

fn check_count<T>(grid: &KeyformGrid<T>) -> Result<(), CellError> {
    let expected = grid.expected_len();
    if grid.keyforms.len() != expected {
        return Err(CellError::KeyformCount {
            expected,
            actual: grid.keyforms.len(),
        });
    }
    Ok(())
}

/// パラメータ `parameter` の軸にキー `key` を加える (`docs/editor.md#キーフォームの編集`)。
///
/// - 軸がなければ、キー 1 つの軸を末尾に加える。セルの並びは変わらない。
/// - 軸があれば、新しいセルを既存の格子のその値での補間 (`docs/spec/evaluation.md#1-キーフォームの重み`
///   と同じ重み。キーの範囲の外は端のキーの値) で埋めるので、評価の結果は変わらない。
///
/// 新しいセルは、格子に推定で作ったセル (`KeyformOrigin::is_derived`) があれば `derived` (方法の記録なし。
/// 格子の `inference` の方法で作り直される)、なければ `authored`。
pub(crate) fn add_key<T: KeyformValues>(
    grid: &mut KeyformGrid<T>,
    parameter: &ParamId,
    key: f32,
) -> Result<(), CellError> {
    if !key.is_finite() {
        return Err(CellError::NonFiniteKey {
            parameter: parameter.to_string(),
            value: key,
        });
    }
    check_count(grid)?;
    let Some(axis) = grid.bindings.iter().position(|b| &b.parameter == parameter) else {
        grid.bindings.push(KeyformBinding {
            parameter: parameter.clone(),
            keys: vec![key],
        });
        return Ok(());
    };
    let keys = grid.bindings[axis].keys.clone();
    if keys.contains(&key) {
        return Err(CellError::DuplicateKey {
            parameter: parameter.to_string(),
            value: key,
        });
    }
    let pos = keys.partition_point(|&k| k < key);
    let (seg, t) = locate(&keys, key);
    let n = keys.len();
    let stride: usize = grid.bindings[..axis].iter().map(|b| b.keys.len()).product();
    let outer = grid.keyforms.len() / (stride * n).max(1);
    let new_origin = if grid.origins.iter().any(|o| o.is_derived()) {
        KeyformOrigin::Derived
    } else {
        KeyformOrigin::Authored
    };
    let old_origin = |i: usize| grid.origins.get(i).copied().unwrap_or_default();
    let mut keyforms = Vec::with_capacity(stride * (n + 1) * outer);
    let mut origins = Vec::with_capacity(keyforms.capacity());
    for o in 0..outer {
        for c in 0..=n {
            for s in 0..stride {
                let old = |c: usize| s + stride * (c + n * o);
                if c == pos {
                    let a = &grid.keyforms[old(seg)];
                    let b = &grid.keyforms[old((seg + 1).min(n - 1))];
                    keyforms.push(lerp_keyform(a, b, t));
                    origins.push(new_origin);
                } else {
                    let i = old(if c < pos { c } else { c - 1 });
                    keyforms.push(grid.keyforms[i].clone());
                    origins.push(old_origin(i));
                }
            }
        }
    }
    grid.keyforms = keyforms;
    // `origins` が空 (すべて `authored`) のままでよければ空に保つ。
    if !grid.origins.is_empty() || new_origin == KeyformOrigin::Derived {
        grid.origins = origins;
    }
    grid.bindings[axis].keys.insert(pos, key);
    Ok(())
}

/// パラメータ `parameter` の軸からキー `key` とその断面のセルを取り除く。
/// 最後のキーなら軸ごと取り除く (その断面が残る。パラメータの削除と同じ畳み方)。
pub(crate) fn remove_key<T>(
    grid: &mut KeyformGrid<T>,
    parameter: &ParamId,
    key: f32,
) -> Result<(), CellError> {
    let axis = grid
        .bindings
        .iter()
        .position(|b| &b.parameter == parameter)
        .ok_or_else(|| CellError::UnknownAxis(parameter.to_string()))?;
    let keys = &grid.bindings[axis].keys;
    let index = keys
        .iter()
        .position(|&k| k == key)
        .ok_or_else(|| CellError::UnknownKey {
            parameter: parameter.to_string(),
            value: key,
        })?;
    check_count(grid)?;
    if keys.len() == 1 {
        collapse_axis(grid, parameter, key);
        return Ok(());
    }
    let keep: Vec<bool> = slice_mask(grid, axis, index)
        .into_iter()
        .map(|removed| !removed)
        .collect();
    retain_cells(&mut grid.keyforms, &keep);
    if !grid.origins.is_empty() {
        retain_cells(&mut grid.origins, &keep);
    }
    grid.bindings[axis].keys.remove(index);
    Ok(())
}

/// セル `cell` を `value` で置き換え、`authored` にする。頂点 (制御点) の数は変えられない。
pub(crate) fn set_cell<T: KeyformValues>(
    grid: &mut KeyformGrid<T>,
    cell: &KeyformCell,
    value: T,
) -> Result<(), CellError> {
    let index = cell_index(grid, cell)?;
    let expected = grid.keyforms[index].shape_len();
    if value.shape_len() != expected {
        return Err(CellError::ShapeMismatch {
            expected,
            actual: value.shape_len(),
        });
    }
    grid.keyforms[index] = value;
    set_authored(grid, index);
    Ok(())
}

/// 1 軸の位置 `(seg, t)` (`docs/spec/evaluation.md#1-軸の位置`)。`keys` は空でないこと。
fn locate(keys: &[f32], v: f32) -> (usize, f32) {
    let last = keys.len().saturating_sub(1);
    if last == 0 || v <= keys[0] {
        return (0, 0.0);
    }
    if v >= keys[last] {
        return (last, 0.0);
    }
    let seg = keys.partition_point(|&k| k <= v) - 1;
    (seg, (v - keys[seg]) / (keys[seg + 1] - keys[seg]))
}

/// `a` と `b` を重み `1 - t`・`t` で補間したキーフォーム。
/// 真偽値のフィールドは重みの大きい方 (同じなら `a`) の値 (`docs/spec/evaluation.md#多重線形の重み`)。
fn lerp_keyform<T: KeyformValues>(a: &T, b: &T, t: f32) -> T {
    if t == 0.0 {
        return a.clone();
    }
    let mut out = if t > 0.5 { b.clone() } else { a.clone() };
    let (mut va, mut vb) = (Vec::new(), Vec::new());
    a.write_values(&mut va);
    b.write_values(&mut vb);
    let values: Vec<f32> = va
        .iter()
        .zip(&vb)
        .map(|(&x, &y)| x * (1.0 - t) + y * t)
        .collect();
    out.read_values(&values);
    out
}

/// セルの指定から、`grid.keyforms` の線形インデックスを求める。
pub(crate) fn cell_index<T>(grid: &KeyformGrid<T>, cell: &KeyformCell) -> Result<usize, CellError> {
    cell_index_in(&grid.bindings, grid.keyforms.len(), cell)
}

/// 軸 `bindings`・セル数 `len` の格子で、セルの指定から線形インデックスを求める。
pub(crate) fn cell_index_in(
    bindings: &[KeyformBinding],
    len: usize,
    cell: &KeyformCell,
) -> Result<usize, CellError> {
    if let Some(unknown) = cell
        .keys()
        .find(|p| !bindings.iter().any(|b| &b.parameter == *p))
    {
        return Err(CellError::UnknownAxis(unknown.to_string()));
    }
    let mut index = 0;
    let mut stride = 1;
    for binding in bindings {
        let value = *cell
            .get(&binding.parameter)
            .ok_or_else(|| CellError::MissingAxis(binding.parameter.to_string()))?;
        let key = binding
            .keys
            .iter()
            .position(|&k| k == value)
            .ok_or_else(|| CellError::UnknownKey {
                parameter: binding.parameter.to_string(),
                value,
            })?;
        index += key * stride;
        stride *= binding.keys.len();
    }
    if index >= len {
        return Err(CellError::OutOfRange { index, len });
    }
    Ok(index)
}

/// セル `index` の由来を `authored` にする (`origins` が空ならすでにすべて `authored`)。
///
/// 格子のセルを手で書き換えるコマンドは、書き換えたセルにこれを呼ぶ
/// (`docs/spec/keyform-inference.md#セルの由来`)。
pub(crate) fn set_authored<T>(grid: &mut KeyformGrid<T>, index: usize) {
    if let Some(origin) = grid.origins.get_mut(index) {
        *origin = KeyformOrigin::Authored;
    }
}

/// 格子を丸ごと置き換える。置き換え前と軸が同じなら、`next` で `derived` とされたセルのうち
/// 値が変わったものを `authored` にする (手で編集した `derived` のセルは `authored` になる)。
pub(crate) fn replace<T: PartialEq>(grid: &mut KeyformGrid<T>, mut next: KeyformGrid<T>) {
    if next.bindings == grid.bindings && next.keyforms.len() == grid.keyforms.len() {
        for index in 0..next.keyforms.len() {
            if next.origin(index).is_derived() && next.keyforms[index] != grid.keyforms[index] {
                set_authored(&mut next, index);
            }
        }
    }
    *grid = next;
}

#[cfg(test)]
mod tests {
    use ugoe_model::KeyformBinding;

    use super::*;

    /// 軸 `A` (キー 0, 1, 2) と軸 `B` (キー -1, 1) の 3×2 格子。セルの値は `10 * b + a`。
    fn grid() -> KeyformGrid<u32> {
        let mut grid = KeyformGrid::constant(0);
        grid.bindings = vec![
            KeyformBinding {
                parameter: "A".into(),
                keys: vec![0.0, 1.0, 2.0],
            },
            KeyformBinding {
                parameter: "B".into(),
                keys: vec![-1.0, 1.0],
            },
        ];
        grid.keyforms = vec![0, 1, 2, 10, 11, 12];
        grid
    }

    #[test]
    fn collapse_first_axis_keeps_slice() {
        let mut g = grid();
        collapse_axis(&mut g, &"A".into(), 1.0);
        assert_eq!(g.bindings.len(), 1);
        assert_eq!(g.bindings[0].parameter, "B".into());
        assert_eq!(g.keyforms, vec![1, 11]);
    }

    #[test]
    fn collapse_also_slices_origins() {
        use ugoe_model::KeyformOrigin::{Authored, Derived};
        let mut g = grid();
        g.origins = vec![Authored, Derived, Authored, Derived, Authored, Derived];
        collapse_axis(&mut g, &"A".into(), 1.0);
        assert_eq!(g.origins, vec![Derived, Authored]);
    }

    #[test]
    fn collapse_last_axis_uses_nearest_key() {
        let mut g = grid();
        // 0 はキーではない。-1 と 1 が等距離なので小さい方 (-1) を採る。
        collapse_axis(&mut g, &"B".into(), 0.0);
        assert_eq!(g.keyforms, vec![0, 1, 2]);
        let mut g = grid();
        collapse_axis(&mut g, &"B".into(), 0.5);
        assert_eq!(g.keyforms, vec![10, 11, 12]);
    }

    #[test]
    fn collapse_unbound_parameter_is_noop() {
        let mut g = grid();
        collapse_axis(&mut g, &"C".into(), 0.0);
        assert_eq!(g, grid());
    }

    #[test]
    fn cell_index_follows_axis_zero_fastest_order() {
        let g = grid();
        let cell: KeyformCell = [("A".into(), 2.0), ("B".into(), 1.0)].into_iter().collect();
        assert_eq!(g.keyforms[cell_index(&g, &cell).unwrap()], 12);

        let missing: KeyformCell = [("A".into(), 2.0)].into_iter().collect();
        assert_eq!(
            cell_index(&g, &missing),
            Err(CellError::MissingAxis("B".into()))
        );
        let unknown_key: KeyformCell = [("A".into(), 0.5), ("B".into(), 1.0)].into_iter().collect();
        assert!(matches!(
            cell_index(&g, &unknown_key),
            Err(CellError::UnknownKey { .. })
        ));
        let extra: KeyformCell = [("A".into(), 0.0), ("B".into(), 1.0), ("C".into(), 0.0)]
            .into_iter()
            .collect();
        assert_eq!(
            cell_index(&g, &extra),
            Err(CellError::UnknownAxis("C".into()))
        );
        assert_eq!(
            cell_index(&KeyformGrid::constant(0), &KeyformCell::new()),
            Ok(0)
        );
    }
}
