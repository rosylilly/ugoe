//! キーフォーム格子の編集の補助。格子の並びは `ugoe_model::KeyformGrid` の doc コメントを参照。

use ugoe_model::{
    KeyformBinding, KeyformGrid, KeyformInference, KeyformInterpolation, KeyformOrigin, ParamId,
};
use ugoe_rigging::KeyformValues;
use ugoe_runtime::ParamIndex;
use ugoe_runtime::keyform::{ResolvedBinding, Weights};

use crate::command::KeyformCell;

/// パラメータ `parameter` の軸を格子から取り除き、キー `value` の断面だけを残す。
///
/// `value` と等しいキーがなければ最も近いキー (等距離なら小さい方) の断面を残す。
/// 深度からの回転の `yaw` / `pitch` がこのパラメータなら外す (回転の中心などの設定は残す)。
/// 軸がなければ何もしない。ブレンドシェイプ格子も同じように畳み、軸がなくなった格子は取り除く
/// (差分が常に 0 になるため)。
pub(crate) fn collapse_axis<T>(grid: &mut KeyformGrid<T>, parameter: &ParamId, value: f32) {
    if grid.blend_shape_index(parameter).is_some() {
        for shape in &mut grid.blend_shapes {
            collapse_axis(shape, parameter, value);
        }
        grid.blend_shapes.retain(|shape| !shape.bindings.is_empty());
    }
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
    // 読み込んだままの (検証していない) 格子では積があふれうるので飽和させる (T0012)。
    let stride = grid.bindings[..axis]
        .iter()
        .fold(1usize, |s, b| s.saturating_mul(b.keys.len().max(1)));
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
/// 新しいセルの由来 (`docs/spec/keyform-inference.md#キーの追加とセルの由来`):
///
/// - キーを加えた後の格子のすべての軸に中立のキー (`default_of` の既定値と等しいキー) があれば、
///   新しいセルを埋めた元のセル (補間した 2 つのセル、範囲の外なら端のセル) のどれかが推定で作ったセル
///   (`KeyformOrigin::is_derived`) なら `derived` (方法の記録なし)、なければ `authored`。
/// - 中立のキーがない軸があれば、格子に推定で作ったセルがあれば `derived`、なければ `authored` (T0608)。
///
/// 組み合わせのセルを推定で作ったセルにするのは呼び出し側 (`crate::keyform::add_key`)。
///
/// 新しいセルの線形インデックスを返す (軸を加えただけなら空)。
pub(crate) fn add_key<T: KeyformValues>(
    grid: &mut KeyformGrid<T>,
    parameter: &ParamId,
    key: f32,
    default_of: impl Fn(&ParamId) -> Option<f32>,
) -> Result<Vec<usize>, CellError> {
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
        return Ok(Vec::new());
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
    // 曲線補間の格子では、新しいセルの形の値を曲線の上に置く (キーを足しても見た目がなるべく変わらないように)。
    let smooth = smooth_axis_weights(grid.interpolation, &keys, key);
    // `check_count` を通ったので、軸のキー数の積 (キーフォーム数) はあふれない。
    let stride: usize = grid.bindings[..axis].iter().map(|b| b.keys.len()).product();
    let outer = grid.keyforms.len() / (stride * n).max(1);
    let mut after = grid.bindings.clone();
    after[axis].keys.insert(pos, key);
    let local = neutral_keys(&after, &default_of).is_some();
    let derived = |derived: bool| {
        if derived {
            KeyformOrigin::Derived
        } else {
            KeyformOrigin::Authored
        }
    };
    let any_derived = grid.origins.iter().any(|o| o.is_derived());
    let old_origin = |i: usize| grid.origins.get(i).copied().unwrap_or_default();
    let mut keyforms = Vec::with_capacity(stride * (n + 1) * outer);
    let mut origins = Vec::with_capacity(keyforms.capacity());
    let mut added = Vec::with_capacity(stride * outer);
    for o in 0..outer {
        for c in 0..=n {
            for s in 0..stride {
                let old = |c: usize| s + stride * (c + n * o);
                if c == pos {
                    let (a, b) = (old(seg), old((seg + 1).min(n - 1)));
                    added.push(keyforms.len());
                    let mut keyform = lerp_keyform(&grid.keyforms[a], &grid.keyforms[b], t);
                    if let Some(weights) = &smooth {
                        let cells = weights.iter().map(|&(k, w)| (&grid.keyforms[old(k)], w));
                        overwrite_shape(&mut keyform, cells);
                    }
                    keyforms.push(keyform);
                    // `t == 0` なら `a` の値そのもの (範囲の外)。
                    let from_derived =
                        old_origin(a).is_derived() || (t != 0.0 && old_origin(b).is_derived());
                    origins.push(derived(if local { from_derived } else { any_derived }));
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
    if !grid.origins.is_empty() || origins.iter().any(|o| o.is_derived()) {
        grid.origins = origins;
    }
    grid.bindings[axis].keys.insert(pos, key);
    Ok(added)
}

/// 中立のキー (パラメータの `default` と等しいキー) の許容差 (`docs/spec/keyform-inference.md#記法`)。
const NEUTRAL_EPSILON: f32 = 1e-5;

/// 軸ごとの中立のキー (パラメータの既定値 `default_of` と等しいキー) のインデックス。
/// 中立のキーがない軸 (既定値のわからないパラメータの軸を含む) があれば `None`。
fn neutral_keys(
    bindings: &[KeyformBinding],
    default_of: impl Fn(&ParamId) -> Option<f32>,
) -> Option<Vec<usize>> {
    bindings
        .iter()
        .map(|b| {
            let default = default_of(&b.parameter)?;
            b.keys
                .iter()
                .position(|&k| (k - default).abs() <= NEUTRAL_EPSILON)
        })
        .collect()
}

/// セル `cells` のうち、中立でない軸が 2 本以上あるセル (組み合わせのセル)。
/// 中立のキーがない軸があれば、どのセルも組み合わせのセルとして扱わない (空を返す)。
pub(crate) fn combination_cells(
    bindings: &[KeyformBinding],
    default_of: impl Fn(&ParamId) -> Option<f32>,
    cells: &[usize],
) -> Vec<usize> {
    let Some(neutral) = neutral_keys(bindings, default_of) else {
        return Vec::new();
    };
    cells
        .iter()
        .copied()
        .filter(|&index| {
            let mut rest = index;
            let mut moved = 0;
            for (binding, &z) in bindings.iter().zip(&neutral) {
                let len = binding.keys.len().max(1);
                if rest % len != z {
                    moved += 1;
                }
                rest /= len;
            }
            moved >= 2
        })
        .collect()
}

/// 基準の姿のセル: 軸ごとにパラメータの既定値に最も近いキー (中立のキーがあればそれ) を採ったセル
/// の線形インデックス。既定値のわからない軸は先頭のキーを採る。
pub(crate) fn rest_cell_index<T>(
    grid: &KeyformGrid<T>,
    default_of: impl Fn(&ParamId) -> Option<f32>,
) -> usize {
    let keys = grid.bindings.iter().map(|binding| {
        default_of(&binding.parameter)
            .and_then(|default| nearest_key(&binding.keys, default))
            .unwrap_or(0)
    });
    let index = linear_index(&grid.bindings, keys);
    index.min(grid.keyforms.len().saturating_sub(1))
}

/// 軸ごとのキーのインデックス `keys` のセルの線形インデックス (最初の軸が最も速く変わる)。
///
/// 読み込んだままの (検証していない) 格子では積があふれうるので飽和させる (T0012)。
/// 飽和した値は本当の値以上なので、キーフォーム数との比べ (範囲の外か) は正しいまま。
fn linear_index(bindings: &[KeyformBinding], keys: impl IntoIterator<Item = usize>) -> usize {
    let mut index = 0usize;
    let mut stride = 1usize;
    for (binding, key) in bindings.iter().zip(keys) {
        index = index.saturating_add(key.saturating_mul(stride));
        stride = stride.saturating_mul(binding.keys.len().max(1));
    }
    index
}

/// 未編集のセル: 由来が `authored` で、基準のセル (すべての軸が中立のキーのセル) ではなく、値が
/// 基準のセルと見た目で変わらない ([`same_values`]) セルの線形インデックス。キーを追加して埋めたまま、
/// まだ形を編集していないセルにあたる (`docs/spec/keyform-inference.md#未編集のセル`)。
/// 中立のキーがない軸があれば空。
pub(crate) fn unedited_cells<T: KeyformValues + PartialEq>(
    grid: &KeyformGrid<T>,
    default_of: impl Fn(&ParamId) -> Option<f32>,
) -> Vec<usize> {
    let Some(neutral) = neutral_keys(&grid.bindings, default_of) else {
        return Vec::new();
    };
    let base = linear_index(&grid.bindings, neutral);
    let Some(rest) = grid.keyforms.get(base) else {
        return Vec::new();
    };
    (0..grid.keyforms.len())
        .filter(|&i| {
            i != base
                && grid.origin(i) == KeyformOrigin::Authored
                && same_values(&grid.keyforms[i], rest)
        })
        .collect()
}

/// 推定の下ごしらえ: セル `cells` だけを `derived` (方法の記録なし) にし、残りのセルをすべて
/// `authored` にする (書き換えずに推定の元にする)。
pub(crate) fn derive_only<T>(grid: &mut KeyformGrid<T>, cells: &[usize]) {
    grid.origins = vec![KeyformOrigin::Authored; grid.keyforms.len()];
    for &index in cells {
        if let Some(origin) = grid.origins.get_mut(index) {
            *origin = KeyformOrigin::Derived;
        }
    }
}

/// セル `cells` のうち、`inferred` (同じ軸の格子で推定したもの) の値が今の値と見た目で変わらない
/// ([`same_values`]) セルを、推定した値と由来に置き換える。
pub(crate) fn adopt_inferred<T: KeyformValues + PartialEq>(
    grid: &mut KeyformGrid<T>,
    inferred: &KeyformGrid<T>,
    cells: &[usize],
) {
    for &index in cells {
        let (Some(current), Some(value)) = (grid.keyforms.get(index), inferred.keyforms.get(index))
        else {
            continue;
        };
        if !same_values(current, value) {
            continue;
        }
        if grid.origins.is_empty() {
            grid.origins = vec![KeyformOrigin::Authored; grid.keyforms.len()];
        }
        grid.keyforms[index] = value.clone();
        grid.origins[index] = inferred.origin(index);
    }
}

/// 2 つのキーフォームが同じ見た目になるか: 数値のフィールドが許容誤差 (値の大きさに対して 1e-4) で
/// 一致し、それ以外のフィールド (真偽値など) が等しい。
fn same_values<T: KeyformValues + PartialEq>(a: &T, b: &T) -> bool {
    let (mut va, mut vb) = (Vec::new(), Vec::new());
    a.write_values(&mut va);
    b.write_values(&mut vb);
    if va.len() != vb.len() {
        return false;
    }
    let close = va
        .iter()
        .zip(&vb)
        .all(|(&x, &y)| (x - y).abs() <= 1e-4 * x.abs().max(y.abs()).max(1.0));
    // 数値を `b` の値にそろえて、残りのフィールドを比べる。
    let mut probe = a.clone();
    probe.read_values(&vb);
    close && probe == *b
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

/// 格子の、パラメータがすべて既定値 (`default_of`。わからなければ最初のキー) のときの補間値 (基準の形)。
/// キーの範囲の外の既定値は端のキーの値 (`docs/spec/evaluation.md#1-軸の位置`)。
pub(crate) fn rest_keyform<T: KeyformValues>(
    grid: &KeyformGrid<T>,
    default_of: impl Fn(&ParamId) -> Option<f32>,
) -> Result<T, CellError> {
    check_count(grid)?;
    let mut rest = KeyformGrid::new(grid.bindings.clone(), grid.keyforms.clone());
    for binding in &grid.bindings {
        let Some(value) = default_of(&binding.parameter).or(binding.keys.first().copied()) else {
            collapse_axis(&mut rest, &binding.parameter, 0.0);
            continue;
        };
        if !binding.keys.contains(&value) {
            add_key(&mut rest, &binding.parameter, value, |_| None)?;
        }
        collapse_axis(&mut rest, &binding.parameter, value);
    }
    rest.keyforms.pop().ok_or(CellError::KeyformCount {
        expected: 1,
        actual: 0,
    })
}

/// ブレンドシェイプのパラメータ `parameter` の軸にキー `key` を加える
/// (`docs/editor.md#ブレンドシェイプ`)。
///
/// そのパラメータを軸に持つブレンドシェイプ格子があれば、その格子に [`add_key`] で加える。
/// なければ、軸がそのパラメータ 1 本で、キーが既定値と `key`、セルがすべて通常の格子の基準の形
/// ([`rest_keyform`]) のブレンドシェイプ格子を末尾に加える (差分は 0 なので見た目は変わらない)。
pub(crate) fn add_blend_shape_key<T: KeyformValues>(
    grid: &mut KeyformGrid<T>,
    parameter: &ParamId,
    key: f32,
    default_of: impl Fn(&ParamId) -> Option<f32>,
) -> Result<(), CellError> {
    if let Some(index) = grid.blend_shape_index(parameter) {
        return add_key(&mut grid.blend_shapes[index], parameter, key, default_of).map(drop);
    }
    let mut shape = KeyformGrid::constant(rest_keyform(grid, &default_of)?);
    add_key(&mut shape, parameter, key, &default_of)?;
    if let Some(default) = default_of(parameter).filter(|&d| d != key) {
        add_key(&mut shape, parameter, default, &default_of)?;
    }
    grid.blend_shapes.push(shape);
    Ok(())
}

/// ブレンドシェイプのパラメータ `parameter` の軸からキー `key` を取り除く ([`remove_key`])。
/// 軸がなくなったブレンドシェイプ格子は取り除く。
pub(crate) fn remove_blend_shape_key<T>(
    grid: &mut KeyformGrid<T>,
    parameter: &ParamId,
    key: f32,
) -> Result<(), CellError> {
    let index = grid
        .blend_shape_index(parameter)
        .ok_or_else(|| CellError::UnknownAxis(parameter.to_string()))?;
    remove_key(&mut grid.blend_shapes[index], parameter, key)?;
    if grid.blend_shapes[index].bindings.is_empty() {
        grid.blend_shapes.remove(index);
    }
    Ok(())
}

/// セルの指定 `cell` が指す格子。ブレンドシェイプのパラメータ (`is_blend_shape`) を含む指定は、
/// そのパラメータを軸に持つブレンドシェイプ格子を指し、含まなければ通常の格子を指す。
pub(crate) fn cell_grid_mut<'a, T>(
    grid: &'a mut KeyformGrid<T>,
    cell: &KeyformCell,
    is_blend_shape: impl Fn(&ParamId) -> bool,
) -> Result<&'a mut KeyformGrid<T>, CellError> {
    let Some(parameter) = cell.keys().find(|p| is_blend_shape(p)) else {
        return Ok(grid);
    };
    let index = grid
        .blend_shape_index(parameter)
        .ok_or_else(|| CellError::UnknownAxis(parameter.to_string()))?;
    Ok(&mut grid.blend_shapes[index])
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

/// 曲線補間の格子 (`interpolation`) で、キー `keys` の軸の値 `v` での 1 軸の重み (キーのインデックス, 重み)。
/// 線形の格子、キーが 2 つ以下の軸、キーの上と範囲の外では `None` (線形の補間と同じ)。
fn smooth_axis_weights(
    interpolation: KeyformInterpolation,
    keys: &[f32],
    v: f32,
) -> Option<Weights> {
    let binding = ResolvedBinding::new(ParamIndex(0), keys.to_vec(), interpolation);
    if !binding.is_smooth() || locate(keys, v).1 == 0.0 {
        return None;
    }
    let mut weights = Weights::new();
    ugoe_runtime::keyform::weights(std::slice::from_ref(&binding), &[v], &mut weights);
    Some(weights)
}

/// `keyform` の形の値 (`KeyformValues::smooth_len`) を、セル `cells` の値を重みで足した値で上書きする
/// (`docs/spec/evaluation.md#曲線補間`)。形以外の値はそのまま。
fn overwrite_shape<'a, T: KeyformValues + 'a>(
    keyform: &mut T,
    cells: impl Iterator<Item = (&'a T, f32)>,
) {
    let len = keyform.smooth_len();
    let mut values = Vec::new();
    keyform.write_values(&mut values);
    values[..len].fill(0.0);
    let mut cell_values = Vec::new();
    for (cell, w) in cells {
        cell_values.clear();
        cell.write_values(&mut cell_values);
        for (v, c) in values[..len].iter_mut().zip(&cell_values) {
            *v += c * w;
        }
    }
    keyform.read_values(&values);
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
    let keys = bindings
        .iter()
        .map(|binding| {
            let value = *cell
                .get(&binding.parameter)
                .ok_or_else(|| CellError::MissingAxis(binding.parameter.to_string()))?;
            binding
                .keys
                .iter()
                .position(|&k| k == value)
                .ok_or_else(|| CellError::UnknownKey {
                    parameter: binding.parameter.to_string(),
                    value,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let index = linear_index(bindings, keys);
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
        assert_eq!(g.bindings[0].parameter, "B");
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

    /// キー 2 つの軸が 70 本 (セル数 2^70 は usize を超える) で、キーフォームが 1 つの不正な格子。
    /// エディタは読み込んだファイルを検証しないので、こうした格子でもパニックしない (T0012)。
    fn huge_grid<T: Clone>(keyform: T) -> KeyformGrid<T> {
        let mut grid = KeyformGrid::constant(keyform);
        grid.bindings = (0..70)
            .map(|i| KeyformBinding {
                parameter: format!("P{i}").as_str().into(),
                keys: vec![0.0, 1.0],
            })
            .collect();
        grid
    }

    #[test]
    fn grids_whose_cell_count_overflows_do_not_panic() {
        let last: ParamId = "P69".into();
        let mut g = huge_grid(0u32);
        collapse_axis(&mut g, &last, 1.0);
        assert_eq!(g.bindings.len(), 69);

        let g = huge_grid(0u32);
        assert_eq!(rest_cell_index(&g, |_| Some(1.0)), 0);
        let far: KeyformCell = g
            .bindings
            .iter()
            .map(|b| (b.parameter.clone(), 1.0))
            .collect();
        assert_eq!(
            cell_index(&g, &far),
            Err(CellError::OutOfRange {
                index: usize::MAX,
                len: 1
            })
        );
        let part = ugoe_model::part::PartKeyform { opacity: 1.0 };
        assert!(matches!(
            add_key(&mut huge_grid(part), &last, 0.5, |_| Some(0.0)),
            Err(CellError::KeyformCount { .. })
        ));

        let g = huge_grid(part);
        assert!(unedited_cells(&g, |_| Some(1.0)).is_empty());
    }
}
