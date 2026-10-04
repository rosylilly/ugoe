---
id: T0015
title: 検証エラーの列の取り出し方と見せ方をそろえる (FormatError・SDK・C ABI・wasm・CLI)
status: done
milestone: M9
crates: [ugoe-format, ugoe, ugoe-ffi, ugoe-wasm, ugoe-cli, ugoe-mcp, docs]
depends_on: []
assignee: claude-worker-T0015
---

# T0015: 検証エラーの列の取り出し方と見せ方をそろえる (FormatError・SDK・C ABI・wasm・CLI)

## 目的

M9 のコードレビューの横断の所見 (T0137・T0530・T0667。まとめは docs/workflow/code-review.md#m9-の結果)。

- `FormatError` には「エラーの列」を持つバリアントが 3 つ (`Invalid`・`InvalidModule`・`InvalidPhysics`) あり、読む側 (CLI・エディタ・ビューア・MCP) が
  一部だけを展開して残りを件数だけで見せていた (T0530)。
- 検証エラーの文言が、`{0:?}` で列を出す (`ugoe/src/dress_up.rs` の物理の検証)、`; ` で並べる (C ABI・wasm・`ugoe-runtime`)、件数だけ (`ugoe-format`) と混在している (T0137)。
- コードを持つエラーを `From` で文字列に畳むと ADR 0029 のコードが失われる (T0667。editor-core は T0678 で扱う)。

## 受け入れ条件

- [x] `FormatError` に検証エラーの列を返すメソッド (例: `validation_errors()`) を足し、読む側がそれを使う
- [x] 検証エラーの列の文言の形を 1 つに決め (並べ方・上限の件数)、SDK・C ABI・wasm・CLI・MCP で揃えた (テストで固定する)
- [x] ツール層・ランタイム層の `From` の変換で、コードを持つエラーを文字列に畳んでいる所が残っていないかを確かめた (editor-core は T0678)
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0137・T0530・T0667 のメモの「横断」、ADR 0029

## メモ

### やったこと

- **文の形:** `{subject} failed validation with {件数} error(s): {1 件目}; {2 件目}; …; and {残り} more`。各エラーは `Display`、
  並べるのは先頭の 10 件 (`DESCRIBED_ERRORS_MAX`) まで。形は `docs/spec/model.md#検証エラーの文` に書いた。
  件数だけ (`ugoe-format`)・`; ` だけ (C ABI・wasm・runtime)・Debug (`DressUpError::Physics`) の混在をこれ 1 つにした。
- **共通の置き場所:** `ugoe_model::validate::describe_errors` (と `DESCRIBED_ERRORS_MAX`)。`ugoe-runtime` は `ugoe-format` に依存しないので、
  両方から届く `ugoe-model` に置いた (写しを作らないため)。`crates:` の外 (`ugoe-model`) への **追加だけの** 公開 API で、型は変えていない。
- `FormatError::validation_errors() -> Option<&[ValidationError]>` を足した (5 つの種類すべて)。`FormatError` の 5 つの種類の `Display` も共通の形にした
  (MCP・C ABI・wasm・Godot はこの `Display` をそのまま見せるので、件数だけでなくエラーが見えるようになった)。
- 揃えた場所: `ugoe-format` (`FormatError`)、`ugoe-runtime` (`CompileError::Invalid`・`ComposeError::Invalid`。`join_errors` を消した)、
  SDK (`DressUpError::Physics`。本体の設定の検査でも使うので subject は `physics settings`)、C ABI と wasm と Godot の物理の検査、
  `ugoe-headless` の物理の検査、CLI の `sample` の検査 (Debug の表記だった)。
- CLI (`read_container`) は `validation_errors()` を使い、各エラーを上限なしに `error: ...` で 1 行ずつ書いて、最後は `<path>: N validation error(s)`
  (列を繰り返さない。`validate` のモデルの検査の最後の文と同じ形)。README を更新。
- テスト: `describe.rs` (上限の前後・空)、`ugoe-format` (5 種類の `validation_errors` と文)、SDK (`invalid_base_physics_changes_nothing`)、
  C ABI (`physics_settings_can_be_replaced` で `ugoe_last_error` を比べる)、wasm、headless、CLI (`validate_lists_physics_errors_in_a_bundle`)、
  MCP (`reports_validation_errors_in_the_shared_form`)。
- `Display` の文は「安定した値として扱わない」(`DressUpError` の doc) ので、公開 API の意味の変更ではないと判断した。

### `From` の変換の確認

`impl From<` と `map_err(... to_string())` をランタイム層・ツール層・アプリの Rust で見た。コードを持つエラーを文字列に畳んでいるのは次だけ。

- `ugoe-editor-core` の `From<ModuleError> for EditError` (T0678 で扱う。触っていない)
- `ugoe-wasm` の `carry_over`: 物理を引き継げなかったとき `AnimationError` (中の `DressUpError` はコードを持つ) を `LoadWarning::PhysicsRemoved { reason }`
  の文に畳む。警告自体は `physics_removed` のコードを持つので見送った (起票の候補)
- ほか (`ugoe-ffi` の `CompileError`・`FormatError`、`EditorError::io` など) は元のエラーがコードを持たない

### やらなかったこと

- `ugoe-editor-core` の `ProjectError::InvalidModule` の文 (`the module is invalid: a; b`) と `message_codes.rs` の `validation_args` (`; ` で全件)、
  `FormatError::InvalidMotion`・`InvalidExpression` が `format.corrupt` に落ちること。editor-core は T0678 が触っており、コードの一覧の変更
  (ADR 0029) になるので触らなかった (起票の候補)。`detail` には新しい `Display` (エラーの列) が入るようになった
- `just ci` の `web-check` が `apps/editor` の `ToolOverlay.test.ts > deletes the selected vertices with Delete` で失敗した
  (`expected [ +0 ] to deeply equal []`)。このブランチは `apps/`・`packages/` を触っておらず、main の T0693・T0699 の組み合わせで
  単体でも再現する (`npx vitest run src/lib/viewport/ToolOverlay.test.ts`)。`web-check` より前 (fmt・clippy・全テスト・layers・refs・groups・
  licenses・publish-check・spec) と後 (`i18n`・`app-build-check`・`script-check`) は通り、`just wasm-check` も通った
- Godot は `cargo clippy --manifest-path bindings/godot/Cargo.toml --all-targets -- -D warnings` だけ確かめた (`just godot-check` は Godot が要るので流していない)

### 起票の候補

- editor-core: `ProjectError::InvalidModule` の文を `describe_errors` に揃え、`FormatError::InvalidMotion`・`InvalidExpression` にコード
  (`format.invalid_motion`・`format.invalid_expression`) を付け、`message_codes.rs` が `FormatError::validation_errors` で引数を作る (T0678・T0163 の後)
- wasm の `carry_over` の `PhysicsRemoved` に、元の `DressUpError` のコードを補足の値として載せる
- `apps/editor` の `ToolOverlay.test.ts` の失敗 (main で再現)

