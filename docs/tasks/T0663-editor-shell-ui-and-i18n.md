---
id: T0663
title: エディタ: 外枠 (メニュー・ツールバー・ダイアログ・ヘルプ) の共有部品への移行と日本語・英語
status: todo
milestone: M8
crates: [apps/editor, docs]
depends_on: [T0901, T0903, T0908]
assignee:
---

# T0663: エディタ: 外枠 (メニュー・ツールバー・ダイアログ・ヘルプ) の共有部品への移行と日本語・英語

## 目的

エディタの外枠を @ugoe/ui と @ugoe/i18n に移す。パネルは T0664・T0665 で行う。

## 受け入れ条件

- [ ] App.svelte・FileMenu・各ダイアログ・ヘルプ・ショートカット・知らせの帯を共有部品に置き換え、文言をカタログに移す (日本語の訳を付ける)
- [ ] 設定に言語の切り替え (既定は OS の言語) を置き、覚える
- [ ] パネルの移行で使う、エディタの中の共通の形 (パネルの見出し・行のレイアウト) を決めて docs/ui.md に書く
- [ ] `just ci`・`cargo check -p ugoe-editor` が通る

## 参考

- docs/editor.md、docs/ui.md

## メモ
