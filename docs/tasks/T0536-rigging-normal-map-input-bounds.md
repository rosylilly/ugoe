---
id: T0536
title: 法線マップの生成で、大きすぎる格子と長さの合わない入力を拒む
status: todo
milestone: M9
crates: [ugoe-rigging, ugoe-ai, docs]
depends_on: [T0526]
assignee:
---

# T0536: 法線マップの生成で、大きすぎる格子と長さの合わない入力を拒む

## 目的

T0526 (ugoe-rigging のコードレビュー) で見つかった、法線マップの生成 (`crates/ugoe-rigging/src/normal.rs`・`normal/group.rs`) の
入力の検査の穴を直す。どれもエラーの型 (`NormalMapError`) か公開の関数の形を変えるので、レビューの中では直さなかった。

1. **まとまりの格子の大きさに上限がない (中):** `GroupField::build` は、置いた画素の位置の外接矩形を、最も細かいテクスチャの
   `canvas_per_texel` の升目に分けて格子を作る。遠く離れた位置 (キャンバスの外に置いた隠しパーツの頂点など) や、極端に小さい
   `canvas_per_texel` があると `w × h` が巨大になり、確保に失敗して止まる (またはとても遅い)。`w as u32` / `h as u32` は
   切り捨てになりうる。升目の数に上限を設け、超えたらエラー (新しい `NormalMapError` の種類) にする。
2. **公開の型と関数が長さを検査しない (低):** `HeightField` は公開のフィールドを持ち、`smooth`・`to_normal_map` は
   `values`・`mask` の長さが `width × height` であることを前提に添字を引く (合わなければパニック)。`HeightField::add` は大きさだけを
   比べて `values` の長さを比べない。`fill_missing` と `NormalMap::extend_edges` も `mask` の長さを検査しない。
   コンストラクタを通す・`Result` を返す・長さが合わなければ何もしない、のいずれかに揃える (呼び出し側は `ugoe-ai::normal`)。

## 受け入れ条件

- [ ] 巨大な格子になる入力 (離れた位置・極端に小さい `canvas_per_texel`) が、確保の前にエラーになるテストがある
- [ ] 長さの合わない `HeightField` などでパニックしないテストがある
- [ ] `ugoe-ai` の呼び出し側と `docs/ai-integration.md#法線マップの生成` を合わせて直した
- [ ] `just ci` が通る

## 参考

- T0526 のメモ (所見の表)
- docs/ai-integration.md#法線マップの生成、docs/workflow/code-review.md

## メモ
