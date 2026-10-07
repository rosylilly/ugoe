//! ID から添字への表 (T0146)。毎フレームの経路で ID を引いても線形探索にならないように、
//! コンパイル時 (と `ParameterValues` の作成時) に作って持ち続ける。
//!
//! 表は `BTreeMap` (T0181)。`std` の `HashMap` は no_std に無く、決まったハッシュ関数の表はファイルから来る ID で
//! 衝突を狙われうる。引き当ては O(log n) で、300 個の表で 1 回 20 ns ほど (`examples/id_lookup_timing.rs`)。

use alloc::boxed::Box;
use alloc::collections::BTreeMap;

/// ID → 添字の表。同じ ID が複数あれば先頭の添字を返す (線形探索と同じ結果)。
#[derive(Debug, Clone, Default)]
pub(crate) struct IdIndex(BTreeMap<Box<str>, u32>);

impl IdIndex {
    /// `ids` の並びの添字で作る。
    pub(crate) fn new<'a>(ids: impl Iterator<Item = &'a str>) -> Self {
        let mut map = BTreeMap::new();
        for (i, id) in ids.enumerate() {
            map.entry(Box::from(id)).or_insert(i as u32);
        }
        Self(map)
    }

    /// ID が `id` の要素の添字。なければ `None`。
    pub(crate) fn get(&self, id: &str) -> Option<u32> {
        self.0.get(id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_ids_resolve_to_the_first() {
        let index = IdIndex::new(["a", "b", "a"].into_iter());
        assert_eq!(index.get("a"), Some(0));
        assert_eq!(index.get("b"), Some(1));
        assert_eq!(index.get("c"), None);
    }
}
