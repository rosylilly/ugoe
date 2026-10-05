---
id: T0561
title: 格子のセルと線形の添字の計算を 1 か所にまとめ、module_rig の引数を文脈の構造体にまとめる
status: in-progress
milestone: M9
crates: [ugoe-rigging, ugoe-editor-core, ugoe-ai]
depends_on: []
assignee: claude-worker-T0561
---

# T0561: 格子のセルと線形の添字の計算を 1 か所にまとめ、module_rig の引数を文脈の構造体にまとめる

## 目的

T0549 の候補。

- 格子の補助 (セル ↔ 線形の添字・キーの比べ方) が `ugoe-editor-core`・`ugoe-ai`・`ugoe-rigging` の 3 か所に同じ並びの計算として書かれている。
  `ugoe-rigging` は editor-core に依存できない (editor-core と ugoe-ai が rigging に依存する) ので、置き場所は `ugoe-rigging` か、より下の層にする。
- `crates/ugoe-ai/src/module_rig.rs` の `candidates` (約 110 行) と `rig_sway` (8 引数) が長い。モジュールのリグの文脈 (`context`・`layout`・`params`・`options`) を
  1 つの構造体にまとめて引数を減らす。

## 受け入れ条件

- [ ] 格子の補助が 1 か所にあり、3 つのクレートの写しがそれを使う。補助にテストがある
- [ ] `candidates`・`rig_sway` が文脈の構造体を受け、clippy の `too_many_arguments` の許可を外せた
- [ ] 振る舞いは変えない (既存のテストがそのまま通る)
- [ ] `just layers` と `just ci` が通る

## 参考

- T0549 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0549 の 2 つ)。
