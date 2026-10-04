---
id: T0167
title: ugoe-runtime の ComposeError に code() を足し、SDK と editor-core の合成のエラーのコードをそこから作る
status: todo
milestone: M9
crates: [ugoe-runtime, ugoe, ugoe-editor-core, docs]
depends_on: []
assignee:
---

# T0167: ugoe-runtime の ComposeError に code() を足し、SDK と editor-core の合成のエラーのコードをそこから作る

## 目的

T0163 で検証・推定のエラーに `code()` を足したが、`ugoe-runtime` の `ComposeError` (`crates/ugoe-runtime/src/compose.rs`) には無い。
SDK の `DressUpError::info` と editor-core の `compose.*` が、同じ名前を 2 か所で手で揃えている (T0163 の候補)。

## 受け入れ条件

- [ ] `ComposeError::code() -> &'static str` (バリアントの名前の `snake_case`) があり、すべてのバリアントにコードがあるテストがある
- [ ] SDK の `DressUpError` と editor-core の `compose.*` がこのコードを使い、既存のコードの値が変わらないことをテストで確かめた
- [ ] docs (`docs/spec/` の該当箇所・editor-core の README) を更新した
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0163 のメモ (起票の候補)
- crates/ugoe/src/dress_up.rs (`info`)、crates/ugoe-editor-core/src/message_codes.rs

## メモ

- T0026 で起票した (元の候補: T0163 の `ComposeError::code()`)。
