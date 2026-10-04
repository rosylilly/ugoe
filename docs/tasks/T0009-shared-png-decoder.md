---
id: T0009
title: PNG のデコードと大きさの上限を 1 か所にまとめる (ugoe-headless と ugoe-wasm)
status: in-progress
milestone: M9
crates: [ugoe-format, ugoe-headless, ugoe-wasm, docs]
depends_on: []
assignee: claude-worker-T0009
---

# T0009: PNG のデコードと大きさの上限を 1 か所にまとめる (ugoe-headless と ugoe-wasm)

## 目的

M9 のコードレビューの横断の所見 (T0212・T0424。まとめは docs/workflow/code-review.md#m9-の結果)。
`crates/ugoe-wasm/src/load.rs` の `decode_png`・`decode_png_premultiplied` は `crates/ugoe-headless/src/image.rs` の写しで、
「見出しの大きさだけで数 GB を確保する」誤りが両方にあり、T0212 と T0424 で別々に直した (一辺の上限 16384 も 2 か所に定数がある)。
次に直すときも片方だけになりやすい。

ランタイム層で使えるクレート (`ugoe-format` は既に `png` に依存し、バイト列の処理はランタイム層の規則の例外) に、
上限つきのデコード (色の正規化・乗算済みアルファの有無) を 1 つ置き、両方から使う。

## 受け入れ条件

- [ ] PNG のデコード (8bit RGBA への正規化・乗算済みアルファ・一辺の上限) を 1 か所に置き、`ugoe-headless` と `ugoe-wasm` がそれを使う
- [ ] 置き場所が公開 API になるなら doc コメントと README を書き、上限を差し替えて試せる形 (T0138 の `payload_within` のように) にする
- [ ] 見出しが巨大な PNG・グレースケール・パレット・16bit の既存のテストが、まとめた側で通る
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0212・T0424 のメモ

## メモ
