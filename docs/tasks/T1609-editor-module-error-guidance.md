---
id: T1609
title: "エディタ: モジュールの適用のエラーに直し方の案内を出し、keep_look の作り直しの警告を捨てない"
status: in-progress
milestone: M9
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T1609
---

# T1609: エディタ: モジュールの適用のエラーに直し方の案内を出し、keep_look の作り直しの警告を捨てない

## 目的

- T0678 で `EditError::Module` とコード (`module.collider_needs_base_physics` など) を足したが、画面は文を出すだけで直し方 (本体を開くなど) を案内しない (T0678 の候補)。
- `Document::set_module_settings` (`crates/ugoe-editor-core/src/document.rs`) は `keep_look` のときの `inference::rederive` の警告 (`RederiveWarning`) を捨て、
  `apply_with_report` は空の報告を返す (T0667 の所見、T0678 の候補)。

## 受け入れ条件

- [ ] モジュールの適用のエラーのうち直し方があるものに、案内 (文と、できれば本体を開くなどの操作) を出す。どのコードに案内を付けたかを docs/editor.md に書いた
- [ ] `set_module_settings` の `keep_look` の作り直しの警告が `ApplyReport` で返り、画面に出る。editor-core のテストがある
- [ ] `en.json`・`ja.json` に訳があり、`just i18n` が通る
- [ ] `just ci` が通る

## 参考

- T0678・T0667 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0678 の案内、T0678/T0667 の `keep_look` の警告)。
