//! キーフォームの格子の補助 (キーの比べ方と、セルと線形インデックスの変換)。格子の並びは
//! `ugoe_model::KeyformGrid` の doc コメントを参照 (最初の軸が最も速く変わる)。

use ugoe_editor_core::{KeyformCell, KeyformTarget};
use ugoe_model::{DeformerKind, KeyformBinding, Model, ParamId};

/// キーどうしを等しいとみなす誤差 (`ugoe-rigging` の中立のキーの判定と同じ)。
pub(crate) const KEY_EPSILON: f32 = 1e-5;

/// `keys` のうち `value` と等しい (誤差 [`KEY_EPSILON`] 以内の) 最初のキーの位置。
pub(crate) fn key_position(keys: &[f32], value: f32) -> Option<usize> {
    keys.iter().position(|k| (k - value).abs() <= KEY_EPSILON)
}

/// `keys` に `value` と等しい (誤差 [`KEY_EPSILON`] 以内の) キーがあるか。
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
pub(crate) fn cell_index(
    bindings: &[KeyformBinding],
    value_of: impl Fn(&ParamId) -> f32,
) -> Result<(usize, KeyformCell), (ParamId, f32)> {
    let mut index = 0;
    let mut stride = 1;
    let mut cell = KeyformCell::new();
    for binding in bindings {
        let value = value_of(&binding.parameter);
        let key = key_position(&binding.keys, value).ok_or((binding.parameter.clone(), value))?;
        index += key * stride;
        stride *= binding.keys.len();
        cell.insert(binding.parameter.clone(), binding.keys[key]);
    }
    Ok((index, cell))
}

/// 線形インデックス `index` のセル。キーのない軸 (検証を通らない格子) があれば `None`。
pub(crate) fn cell_at(bindings: &[KeyformBinding], index: usize) -> Option<KeyformCell> {
    let mut rest = index;
    let mut cell = KeyformCell::new();
    for binding in bindings {
        let n = binding.keys.len();
        let key = *binding.keys.get(rest.checked_rem(n)?)?;
        rest /= n;
        cell.insert(binding.parameter.clone(), key);
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
        assert_eq!(key_position(&[-1.0, 0.0, 1.0], 1.0 + 1e-6), Some(2));
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
    fn an_axis_without_keys_has_no_cells() {
        let bindings = [binding("A", &[0.0, 1.0]), binding("B", &[])];
        assert_eq!(cell_at(&bindings, 0), None);
    }
}
