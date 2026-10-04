---
id: T0678
title: モジュールのエラーをコードを保って EditError と ProjectError に渡す
status: in-progress
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

- [ ] `ModuleError` の種類ごとのコードと引数が、`Document::apply` のエラーからも取れる (テストで確かめる)
- [ ] `rebase_module_on` のモジュールでないときのエラーが、ほかと同じ種類になる
- [ ] `.ugoemod` が読めないときのエラーが、書き出しの失敗と区別できる
- [ ] 足したコードを `docs/editor.md#エラーと警告のコード` の一覧と翻訳に書いた
- [ ] `just ci` が通る

## 参考

- docs/adr/0029-shared-ui-and-i18n.md
- crates/ugoe-editor-core/src/message_codes.rs
- T0667 のメモ

## メモ
