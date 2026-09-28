//! キーフォーム格子の編集の補助。格子の並びは `ugoe_model::KeyformGrid` の doc コメントを参照。

use ugoe_model::{KeyformBinding, KeyformGrid, KeyformInference, KeyformOrigin, ParamId};

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
            if next.origin(index) == KeyformOrigin::Derived
                && next.keyforms[index] != grid.keyforms[index]
            {
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
