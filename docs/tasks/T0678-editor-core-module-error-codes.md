---
id: T0678
title: モジュールのエラーをコードを保って EditError と ProjectError に渡す
status: done
milestone: M9
crates: [ugoe-editor-core, ugoe-mcp, apps/editor, docs]
depends_on: [T0667]
assignee: claude-worker-T0678
---

# T0678: モジュールのエラーをコードを保って EditError と ProjectError に渡す

## 目的

T0667 のレビューの所見。エラーのコード (ADR 0029、`Coded`) が、モジュールのプロジェクトの経路で失われている。

- `impl From<ModuleError> for EditError` が、`ReadOnlyBase` 以外の `ModuleError` (`ColliderNeedsBasePhysics`・`UnknownSlot`・`InvalidName`・
  `Compose`) を `EditError::ModuleShape { reason: other.to_string() }` に畳む。コードは `edit.module_shape` になり、引数は英語の文だけになるので、
  `ColliderNeedsBasePhysics` の案内 (本体に物理を足す) などが翻訳できない。
- `Document::rebase_module_on` は、モジュールのプロジェクトでないときに `ModuleError::Shape { reason: "this is not a module project" }` を返す
  (`EditError::NotModule`・`ProjectError::NotAModule` と揃っていない)。
- `compare_with_module_file` は、`.ugoemod` が読めないときに `ProjectError::Export` (「bundle export failed」、`project.export_failed`) を返す。
  読み込みの失敗なのに書き出しの失敗と表示される。

`EditError` に `Module(ModuleError)` を足す (`ModuleShape` は `ModuleError::Shape` に寄せる)・`ModuleError` に `NotModule` を足す・
`ProjectError` に読み込みの失敗の種類を足す、などで直す。公開 API とコードの一覧 (`docs/editor.md#エラーと警告のコード`) が変わるので、
エディタ (`apps/editor`) と `ugoe-mcp` の受け手と翻訳も合わせて直す。**コードは名前を変えない** (足すだけ) 規則に注意する。

## 受け入れ条件

- [x] `ModuleError` の種類ごとのコードと引数が、`Document::apply` のエラーからも取れる (テストで確かめる)
- [x] `rebase_module_on` のモジュールでないときのエラーが、ほかと同じ種類になる
- [x] `.ugoemod` が読めないときのエラーが、書き出しの失敗と区別できる
- [x] 足したコードを `docs/editor.md#エラーと警告のコード` の一覧と翻訳に書いた
- [x] `just ci` が通る

## 参考

- docs/adr/0029-shared-ui-and-i18n.md
- crates/ugoe-editor-core/src/message_codes.rs
- T0667 のメモ

## メモ

- やったこと:
  - `EditError::Module(ModuleError)` を足した (コード・引数・原因は中の `ModuleError` のもの、`module.*`)。`From<ModuleError> for EditError` は
    `ReadOnlyBase` → `EditError::ReadOnlyBase`、`Shape` → `EditError::ModuleShape`、`NotModule` → `EditError::NotModule` に、ほかは
    `Module` に包む。既存のコード (`edit.read_only_base`・`edit.module_shape`・`edit.not_module`) は名前を変えず、出る場面も変えていない。
    `ModuleShape` の英語の文は `ModuleError::Shape` と同じ `cannot be kept as a module: {reason}` にした (以前は `reason` に前置きが入っていた)。
  - `ModuleError::NotModule` (`module.not_module`) を足し、`rebase_module_on` はこれを返す。`ProjectError` への変換は `#[from]` をやめて手で書き、
    `NotModule` → `ProjectError::NotAModule` にする (`EditError` でも `NotModule`)。
  - `ProjectError::UnreadableModuleFile` (`project.module_file_unreadable`、原因は `format.*`) を足し、`compare_with_module_file` が使う。
  - `docs/editor.md#コードの一覧`・`en.json`・`ja.json`・`message_catalog.rs` の見本・`just gen` の `messageCodes.ts` を合わせた。
  - テスト: `module_errors_keep_their_codes_through_apply`・`rebasing_a_model_project_is_not_a_module_error`・`broken_files_are_errors` (直した)。
- やらなかったこと: `ugoe-mcp` とエディタのバックエンド (`apps/editor/src-tauri`) はエラーを `to_message()` で汎用に渡すだけで、
  `ModuleShape` などに合わせた分岐が無いので変更不要だった (`module.rs` の `diagnose` は `ProjectError::Module` の種類で分けており、そのまま)。
  T0667 の所見の `set_module_settings` の `RederiveWarning` を捨てる件は扱っていない。
- 起票の候補: `EditError::Module` を足したので、エディタの画面でモジュールの適用のエラー (`module.collider_needs_base_physics` など) に
  直し方の案内 (本体を開くなど) を出すか検討する。
