//! キーフォームの格子のセルと線形インデックスの変換と、キーの比べ方。
//!
//! 格子の並びは `ugoe_model::KeyformGrid` の doc コメントを参照 (最初の軸が最も速く変わる)。
//! `ugoe-rigging`・`ugoe-editor-core`・`ugoe-ai` はこの並びの計算をここで共有する。
//!
//! 読み込んだままの (検証していない) 格子では、軸の多い格子でキー数の積があふれうる (T0012)。
//! ここの計算はあふれずに飽和させる。飽和した値は本当の値以上なので、キーフォーム数との比べ
//! (範囲の外か) は正しいまま。キーのない軸 (検証を通らない格子) は、キー数 1 の軸として並びを数える。

use ugoe_model::KeyformBinding;

/// キーどうし (中立のキーとパラメータの `default` を含む) を等しいとみなす誤差
/// (`docs/spec/keyform-inference.md#記法`)。
pub const KEY_EPSILON: f32 = 1e-5;

/// `keys` のうち `value` と等しい (誤差 [`KEY_EPSILON`] 以内の) 最初のキーの位置。
pub fn key_position(keys: &[f32], value: f32) -> Option<usize> {
    keys.iter().position(|k| (k - value).abs() <= KEY_EPSILON)
}

/// 軸 `bindings` のキーの数 (軸の順)。
pub fn axis_lens(bindings: &[KeyformBinding]) -> impl Iterator<Item = usize> + Clone + '_ {
    bindings.iter().map(|b| b.keys.len())
}

/// キー数 `lens` の軸の格子のセル数 (キー数の積)。usize に収まらなければ `usize::MAX` に飽和させる。
pub fn cell_count(lens: impl IntoIterator<Item = usize>) -> usize {
    lens.into_iter().fold(1, usize::saturating_mul)
}

/// キー数 `lens` の軸の格子で、軸ごとのキーの位置 `keys` のセルの線形インデックス。飽和させて求める。
pub fn linear_index(
    lens: impl IntoIterator<Item = usize>,
    keys: impl IntoIterator<Item = usize>,
) -> usize {
    let mut index = 0usize;
    let mut stride = 1usize;
    for (len, key) in lens.into_iter().zip(keys) {
        index = index.saturating_add(key.saturating_mul(stride));
        stride = stride.saturating_mul(len.max(1));
    }
    index
}

/// キー数 `lens` の軸の格子で、線形インデックス `index` のセルの軸ごとのキーの位置 (軸の順)。
/// キーのない軸では 0 (その軸にキーはないので、呼び出し側が添字の外を確かめる)。
pub fn cell_coords(
    lens: impl IntoIterator<Item = usize>,
    index: usize,
) -> impl Iterator<Item = usize> {
    let mut rest = index;
    lens.into_iter().map(move |len| {
        let len = len.max(1);
        let key = rest % len;
        rest /= len;
        key
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_equal_within_the_epsilon() {
        assert_eq!(key_position(&[-1.0, 0.0, 1.0], 1.0 + 1e-6), Some(2));
        assert_eq!(key_position(&[-1.0, 0.0, 1.0], 0.5), None);
        assert_eq!(key_position(&[], 0.0), None);
    }

    #[test]
    fn the_first_axis_changes_fastest() {
        let lens = [3, 2];
        assert_eq!(cell_count(lens), 6);
        assert_eq!(linear_index(lens, [1, 1]), 4);
        assert_eq!(cell_coords(lens, 4).collect::<Vec<_>>(), [1, 1]);
        assert_eq!(cell_coords(lens, 2).collect::<Vec<_>>(), [2, 0]);
    }

    #[test]
    fn cells_and_indices_round_trip() {
        let lens = [3, 1, 4, 2];
        for index in 0..cell_count(lens) {
            let coords: Vec<_> = cell_coords(lens, index).collect();
            assert_eq!(linear_index(lens, coords), index);
        }
    }

    #[test]
    fn counts_and_indices_saturate() {
        // セル数 2^70 は usize を超える (読み込んだモデルの格子は検証していないことがある。T0012)。
        let lens = [2; 70];
        assert_eq!(cell_count(lens), usize::MAX);
        assert_eq!(cell_count([]), 1);
        assert_eq!(linear_index(lens, [1; 70]), usize::MAX);
        assert_eq!(
            cell_coords(lens, usize::MAX).take(3).collect::<Vec<_>>(),
            [1, 1, 1]
        );
    }

    #[test]
    fn an_axis_without_keys_counts_as_one_key() {
        let lens = [2, 0, 3];
        assert_eq!(cell_count(lens), 0);
        assert_eq!(linear_index(lens, [1, 0, 2]), 5);
        assert_eq!(cell_coords(lens, 5).collect::<Vec<_>>(), [1, 0, 2]);
    }

    #[test]
    fn axis_lens_follow_the_bindings() {
        let binding = |keys: &[f32]| KeyformBinding {
            parameter: ugoe_model::ParamId::new("A"),
            keys: keys.to_vec(),
        };
        let bindings = [binding(&[0.0, 1.0]), binding(&[-1.0, 0.0, 1.0])];
        assert_eq!(axis_lens(&bindings).collect::<Vec<_>>(), [2, 3]);
    }
}
