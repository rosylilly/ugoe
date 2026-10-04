---
id: T0681
title: エラーと警告のコードと翻訳のカタログの対応を自動で検査する
status: in-progress
milestone: M9
crates: [apps/editor, packages/i18n, docs]
depends_on: [T0668]
assignee: claude-worker-T0681
---

# T0681: エラーと警告のコードと翻訳のカタログの対応を自動で検査する

## 目的

`ugoe-editor-core` が画面に出すエラー・警告のコード (`message_codes.rs`・`import.rs` の `psd.*`・`inference.rs` の
`inference.mirror_failed`・`project.rs`) は、`docs/editor.md#エラーと警告のコード` に載っていることだけがテスト
(`message_tests.rs` の `every_code_is_documented`) で確かめられている。エディタの翻訳のカタログ
(`apps/editor/locales/en.json`・`ja.json` の `errors.<code>`) に文があるか、文の引数 (`{{name}}`) が Rust の
`Coded::args` の名前と一致するかは、どこでも検査していない ([ADR 0029](../adr/0029-shared-ui-and-i18n.md))。

T0668 のレビューで手作業で照合した時点では、すべてのコードに英語と日本語の文があり、引数も一致していた。
コードを足したときにカタログへの追加を忘れると、画面には英語の文 (`message`) が黙って出る。

## 受け入れ条件

- [ ] コードの一覧の出し方を決める (例: `ugoe-editor-core` が既知のコードと引数の名前の一覧を返す関数を持ち、`just gen` で
      `apps/editor/src/lib/generated/` に書き出す。MIT のクレートから GPL のカタログを読まない向きにする)
- [ ] エディタ側のテスト (Vitest か `just i18n`) で、すべてのコードが `en.json` と `ja.json` の `errors` にあることを確かめる
- [ ] 文の引数の名前が、そのコードの引数の名前の部分集合であることを確かめる
- [ ] `just ci` が通る

## 参考

- docs/adr/0029-shared-ui-and-i18n.md
- docs/editor.md#エラーと警告のコード
- packages/i18n/scripts/check-catalogs.mjs
- T0668 のメモ

## メモ
