---
id: T0167
title: ugoe-runtime の ComposeError に code() を足し、SDK と editor-core の合成のエラーのコードをそこから作る
status: done
milestone: M9
crates: [ugoe-runtime, ugoe, ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T0167
---

# T0167: ugoe-runtime の ComposeError に code() を足し、SDK と editor-core の合成のエラーのコードをそこから作る

## 目的

T0163 で検証・推定のエラーに `code()` を足したが、`ugoe-runtime` の `ComposeError` (`crates/ugoe-runtime/src/compose.rs`) には無い。
SDK の `DressUpError::info` と editor-core の `compose.*` が、同じ名前を 2 か所で手で揃えている (T0163 の候補)。

## 受け入れ条件

- [x] `ComposeError::code() -> &'static str` (バリアントの名前の `snake_case`) があり、すべてのバリアントにコードがあるテストがある
- [x] SDK の `DressUpError` と editor-core の `compose.*` がこのコードを使い、既存のコードの値が変わらないことをテストで確かめた
- [x] docs (`docs/spec/` の該当箇所・editor-core の README) を更新した
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0163 のメモ (起票の候補)
- crates/ugoe/src/dress_up.rs (`info`)、crates/ugoe-editor-core/src/message_codes.rs

## メモ

- T0026 で起票した (元の候補: T0163 の `ComposeError::code()`)。
- claude-worker-T0167 のメモ:
  - `ComposeError::code()` を足した (`crates/ugoe-runtime/src/compose.rs`。テスト `compose::tests::codes_are_fixed` で全バリアントの値を固定し、重複が無いことも確かめる)。
    値はバリアントの名前の `snake_case` だが、**`Invalid` だけは既存の値 `invalid_composition` のまま** (SDK・wasm・C ABI・エディタの
    `compose.invalid_composition` が既に使っている値を変えないため。受け入れ条件の「既存のコードの値が変わらない」を優先した)。
  - SDK: `DressUpError::info` の合成の分は `ComposeError::code` を使い、補足の値は `compose_fields` に分けた。既存の
    `every_error_kind_has_a_fixed_code_and_fields` が全種類の値を固定しており、合成の分は `ComposeError::code` と一致することも足した。
  - editor-core: `Coded::code` は `&'static str` を返すので、`compose.` を付けた値は従来どおり `message_codes.rs` に並べたまま
    (T0163 の `infer.*` などと同じ形)。`source_error_codes_follow_the_crate_codes` で、見本 (`compose_errors`。全 13 種) のコードが
    `compose.{ComposeError::code}` と一致することを確かめる。手で揃える箇所は残るが、食い違えばテストが落ちる。
  - docs: `docs/spec/modules.md#合成` にコードの一覧、`docs/editor.md` の「元のエラーのコード」、`docs/spec/public-spec.md` の備考 (英語版には書かない)、
    editor-core の README。
  - `just ci`・`just wasm-check` は通った。
  - 起票の候補: `Coded::code` を `Cow<'static, str>` などにして、元のクレートのコードに領域の名前を付けて組み立てる (`validation.*`・`infer.*`・
    `world.*`・`normal_map.*`・`compose.*` の手で並べた一覧を無くす)。
