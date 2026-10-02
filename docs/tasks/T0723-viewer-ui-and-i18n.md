---
id: T0723
title: ビューア: 共有部品への移行と日本語・英語の切り替え
status: todo
milestone: M8
crates: [apps/viewer, docs]
depends_on: [T0901, T0903, T0904, T0908]
assignee:
---

# T0723: ビューア: 共有部品への移行と日本語・英語の切り替え

## 目的

ビューアを @ugoe/ui と @ugoe/i18n に移し、日本語と英語を切り替えられるようにする。

## 受け入れ条件

- [ ] 画面の部品を @ugoe/ui に置き換え、独自の部品と CSS を減らす
- [ ] すべての文言をカタログに移し、英語の訳を付ける。設定に言語の切り替え (既定は OS の言語) を置き、覚える。OS の言語は tauri-plugin-os の locale() で取り、@ugoe/i18n の setSystemLanguages() に渡す (WebView の navigator.languages は OS の言語を正しく返さない。利用者の判断、2026-10-02)
- [ ] T0908 の検査で、ビューアの許す一覧が空になる
- [ ] ヘッドレスの Chrome で両方の言語の画面を確かめる。`just ci`・ビューアのビルドが通る

## 参考

- apps/viewer/README.md、docs/ui.md

## メモ
