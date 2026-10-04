---
id: T0015
title: 検証エラーの列の取り出し方と見せ方をそろえる (FormatError・SDK・C ABI・wasm・CLI)
status: todo
milestone: M9
crates: [ugoe-format, ugoe, ugoe-ffi, ugoe-wasm, ugoe-cli, ugoe-mcp, docs]
depends_on: []
assignee:
---

# T0015: 検証エラーの列の取り出し方と見せ方をそろえる (FormatError・SDK・C ABI・wasm・CLI)

## 目的

M9 のコードレビューの横断の所見 (T0137・T0530・T0667。まとめは docs/workflow/code-review.md#m9-の結果)。

- `FormatError` には「エラーの列」を持つバリアントが 3 つ (`Invalid`・`InvalidModule`・`InvalidPhysics`) あり、読む側 (CLI・エディタ・ビューア・MCP) が
  一部だけを展開して残りを件数だけで見せていた (T0530)。
- 検証エラーの文言が、`{0:?}` で列を出す (`ugoe/src/dress_up.rs` の物理の検証)、`; ` で並べる (C ABI・wasm・`ugoe-runtime`)、件数だけ (`ugoe-format`) と混在している (T0137)。
- コードを持つエラーを `From` で文字列に畳むと ADR 0029 のコードが失われる (T0667。editor-core は T0678 で扱う)。

## 受け入れ条件

- [ ] `FormatError` に検証エラーの列を返すメソッド (例: `validation_errors()`) を足し、読む側がそれを使う
- [ ] 検証エラーの列の文言の形を 1 つに決め (並べ方・上限の件数)、SDK・C ABI・wasm・CLI・MCP で揃えた (テストで固定する)
- [ ] ツール層・ランタイム層の `From` の変換で、コードを持つエラーを文字列に畳んでいる所が残っていないかを確かめた (editor-core は T0678)
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0137・T0530・T0667 のメモの「横断」、ADR 0029

## メモ
