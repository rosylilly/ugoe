//! キーフォーム: パラメータ空間の格子点ごとに形状 (等) を持ち、多重線形補間で合成する。
//! 詳細な数式は `docs/spec/evaluation.md` を参照。

use serde::{Deserialize, Serialize};

use crate::ParamId;

/// 1 本のパラメータ軸と、その軸上のキー位置 (昇順・重複なし)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyformBinding {
    pub parameter: ParamId,
    pub keys: Vec<f32>,
}

/// N 本のパラメータ軸の直積格子上に並んだキーフォーム。
///
/// `keyforms.len() == bindings.iter().map(|b| b.keys.len()).product()` (軸 0 本なら 1)。
/// 並びは **軸 0 が最も速く変化する** 行優先 (column-major 的) 順。
/// 格子座標 `(i0, i1, ..)` の線形インデックスは `i0 + n0 * (i1 + n1 * (i2 + ..))`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyformGrid<T> {
    #[serde(default)]
    pub bindings: Vec<KeyformBinding>,
    pub keyforms: Vec<T>,
}

impl<T> KeyformGrid<T> {
    /// パラメータに依存しない、単一キーフォームの格子。
    pub fn constant(value: T) -> Self {
        Self {
            bindings: Vec::new(),
            keyforms: vec![value],
        }
    }

    /// 格子として期待されるキーフォーム数。
    pub fn expected_len(&self) -> usize {
        self.bindings.iter().map(|b| b.keys.len()).product()
    }
}
