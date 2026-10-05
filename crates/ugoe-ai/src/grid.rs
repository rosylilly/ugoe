//! キーフォームの格子の補助 (要素の軸と、セルの指定 [`KeyformCell`] と線形インデックスの変換)。
//! 並びの計算とキーの比べ方は `ugoe_rigging::grid_index` を使う (`ugoe-editor-core` と共有する)。

use ugoe_editor_core::{KeyformCell, KeyformTarget};
use ugoe_model::{DeformerKind, KeyformBinding, Model, ParamId};
pub(crate) use ugoe_rigging::grid_index::cell_count;
use ugoe_rigging::grid_index::{axis_lens, cell_coords, key_position, linear_index};

/// `keys` に `value` と等しい (誤差 `ugoe_rigging::grid_index::KEY_EPSILON` 以内の) キーがあるか。
pub(crate) fn has_key(keys: &[f32], value: f32) -> bool {
    key_position(keys, value).is_some()
}

/// 要素 `target` (メッシュかワープか回転デフォーマ) の格子の軸。要素がなければ `None`。
pub(crate) fn bindings<'a>(
    model: &'a Model,
    target: &KeyformTarget,
) -> Option<&'a [KeyformBinding]> {
    match target {
        KeyformTarget::Mesh(id) => model
            .meshes
            .iter()
            .find(|m| &m.id == id)
            .map(|m| m.keyforms.bindings.as_slice()),
        KeyformTarget::Deformer(id) => {
            model
                .deformers
                .iter()
                .find(|d| &d.id == id)
                .and_then(|d| match &d.kind {
                    DeformerKind::Warp(w) => Some(w.keyforms.bindings.as_slice()),
                    DeformerKind::Rotation(r) => Some(r.keyforms.bindings.as_slice()),
                    DeformerKind::Unknown => None,
                })
        }
        _ => None,
    }
}

/// 要素 `target` の格子の、パラメータ `id` の軸のキー (軸か要素がなければ空)。
pub(crate) fn axis_keys<'a>(model: &'a Model, target: &KeyformTarget, id: &ParamId) -> &'a [f32] {
    bindings(model, target)
        .and_then(|bindings| bindings.iter().find(|b| &b.parameter == id))
        .map_or(&[], |b| b.keys.as_slice())
}

/// 各軸の値を `value_of` で決めたセルの線形インデックスと、格子のキーで書いたセル。
/// 値と等しいキーのない軸があれば、その軸と値を `Err` で返す。
///
/// インデックスは飽和させて求める (飽和した値は本当の値以上なので、キーフォーム数との比べは正しいまま)。
pub(crate) fn cell_index(
    bindings: &[KeyformBinding],
    value_of: impl Fn(&ParamId) -> f32,
) -> Result<(usize, KeyformCell), (ParamId, f32)> {
    let mut keys = Vec::with_capacity(bindings.len());
    let mut cell = KeyformCell::new();
    for binding in bindings {
        let value = value_of(&binding.parameter);
        let key = key_position(&binding.keys, value).ok_or((binding.parameter.clone(), value))?;
        keys.push(key);
        cell.insert(binding.parameter.clone(), binding.keys[key]);
    }
    Ok((linear_index(axis_lens(bindings), keys), cell))
}

/// 線形インデックス `index` のセル。キーのない軸 (検証を通らない格子) があれば `None`。
pub(crate) fn cell_at(bindings: &[KeyformBinding], index: usize) -> Option<KeyformCell> {
    let mut cell = KeyformCell::new();
    for (binding, key) in bindings.iter().zip(cell_coords(axis_lens(bindings), index)) {
        cell.insert(binding.parameter.clone(), *binding.keys.get(key)?);
    }
    Some(cell)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(parameter: &str, keys: &[f32]) -> KeyformBinding {
        KeyformBinding {
            parameter: ParamId::new(parameter),
            keys: keys.to_vec(),
        }
    }

    #[test]
    fn keys_are_equal_within_the_epsilon() {
        assert!(has_key(&[-1.0, 0.0, 1.0], 1.0 + 1e-6));
        assert!(!has_key(&[-1.0, 0.0, 1.0], 0.5));
    }

    #[test]
    fn cells_and_indices_round_trip() {
        let bindings = [binding("A", &[-1.0, 0.0, 1.0]), binding("B", &[0.0, 1.0])];
        for index in 0..6 {
            let cell = cell_at(&bindings, index).unwrap();
            let (back, named) = cell_index(&bindings, |p| cell[p]).unwrap();
            assert_eq!((back, named), (index, cell));
        }
        // 最初の軸が最も速く変わる。
        let cell = cell_at(&bindings, 4).unwrap();
        assert_eq!(cell[&ParamId::new("A")], 0.0);
        assert_eq!(cell[&ParamId::new("B")], 1.0);
    }

    #[test]
    fn a_cell_is_written_with_the_grid_keys() {
        let bindings = [binding("A", &[0.0, 1.0])];
        let (index, cell) = cell_index(&bindings, |_| 1.0 - 1e-6).unwrap();
        assert_eq!(index, 1);
        assert_eq!(cell[&ParamId::new("A")], 1.0);
        assert_eq!(
            cell_index(&bindings, |_| 0.5),
            Err((ParamId::new("A"), 0.5))
        );
    }

    #[test]
    fn cell_counts_and_indices_saturate() {
        // セル数 2^70 は usize を超える (読み込んだモデルの格子は検証していないことがある。T0012)。
        let bindings: Vec<_> = (0..70)
            .map(|i| binding(&format!("P{i}"), &[0.0, 1.0]))
            .collect();
        assert_eq!(
            cell_count(bindings.iter().map(|b| b.keys.len())),
            usize::MAX
        );
        assert_eq!(cell_count([3, 2]), 6);
        assert_eq!(cell_count([]), 1);
        let (index, _) = cell_index(&bindings, |_| 1.0).unwrap();
        assert_eq!(index, usize::MAX);
    }

    #[test]
    fn an_axis_without_keys_has_no_cells() {
        let bindings = [binding("A", &[0.0, 1.0]), binding("B", &[])];
        assert_eq!(cell_at(&bindings, 0), None);
    }
}
