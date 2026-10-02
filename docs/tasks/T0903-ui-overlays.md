---
id: T0903
title: UI: 共有部品の 2 段目 (ダイアログ・知らせ・ツールチップ・メニュー・色の選択・ファイルのドロップ)
status: todo
milestone: M8
crates: [packages, docs]
depends_on: [T0902]
assignee:
---

# T0903: UI: 共有部品の 2 段目 (ダイアログ・知らせ・ツールチップ・メニュー・色の選択・ファイルのドロップ)

## 目的

T0902 の続き。アプリごとに作りが違う、重ねて出す部品を揃える。

## 受け入れ条件

- [ ] bits-ui を土台にする (ADR 0029)。Dialog (モーダル。フォーカスを閉じ込める・Esc で閉じる)、知らせの帯 (Notice / Toast)、Tooltip (IconButton にも付ける)、Menu / Dropdown、Popover、ColorPicker (Popover の中)、ファイルのドロップ領域、ショートカットの表記
- [ ] 見本の画面に足し、スクリーンショットの検査に入れる
- [ ] Vitest のテストと README。`just ci` が通る

## 参考

- apps/editor の BaseChangeDialog.svelte など (いまのダイアログ)、apps/viewer の MemoryDialog.svelte・LicenseDialog.svelte

## メモ
