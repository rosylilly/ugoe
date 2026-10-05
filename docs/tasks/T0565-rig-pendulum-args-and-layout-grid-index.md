---
id: T0565
title: editor-core の Rig::pendulum の引数をまとめ、ugoe-rigging の Layout の添字の計算を grid_index にそろえるか確かめる
status: in-progress
milestone: M9
crates: [ugoe-editor-core, ugoe-rigging]
depends_on: []
assignee: claude-worker-T0565
---

# T0565: editor-core の Rig::pendulum の引数をまとめ、ugoe-rigging の Layout の添字の計算を grid_index にそろえるか確かめる

## 目的

T0561 で格子の添字の計算を `grid_index` にまとめ、`module_rig` の引数を文脈の構造体にまとめた。次が残っている (T0561 の候補)。

1. editor-core の `Rig::pendulum` は 11 個の引数を取る。振り子の設定の構造体にまとめる (振る舞いは変えない)。
2. `ugoe-rigging` の `Layout` の `stride` による添字の計算 (`additive.rs`・`mirror.rs`) も `grid_index` にそろえるか。内側のループなので、性能を測ってから決める。

## 受け入れ条件

- [ ] `Rig::pendulum` の引数が構造体にまとまり、`clippy::too_many_arguments` の許可が要らない。呼ぶ側を直し、振る舞いのテストが今のまま通る
- [ ] `Layout` の添字の計算を `grid_index` にそろえた場合の時間を測り (ベンチか、大きな格子の推定の時間)、そろえるかを決めた。数字と判断をメモに書いた
- [ ] `just ci` が通る

## 参考

- T0561 のメモ (起票の候補)

## メモ

- T0032 で起票した (元の候補: T0561 (2 つ))。
