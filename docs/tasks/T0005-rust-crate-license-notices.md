---
id: T0005
title: 3 つのデスクトップアプリの配布物に、Rust のクレートのライセンスの表記を入れる
status: todo
milestone: Release
crates: [apps/editor, apps/viewer, apps/studio, scripts, justfile, docs]
depends_on: [T0811]
assignee:
---

# T0005: Rust のクレートのライセンスの表記

## 目的

エディタ・ビューア・スタジオの third-party-notices は、同梱する JS と MediaPipe の資産だけを載せていて、バイナリに入る Rust のクレート (MIT・Apache-2.0 など)
のライセンスの表記が無い (T0811 のメモ)。MIT・Apache-2.0 は配布物に表記を入れることを求めるので、配布の前に揃える。

## 受け入れ条件

- [ ] 各アプリのバイナリに入る Rust のクレートと、そのライセンスの全文を、配布物とライセンスの画面に入れる。作り方 (cargo-about などの道具を使うか、
      scripts/check-licenses.mjs を広げて自前で作るか) は、外部の道具を足すなら利用者に確認する
- [ ] 表記を作り直す just のレシピと、依存が変わったのに表記が古いままなら失敗する検査
- [ ] JS の表記の版を lockfile と突き合わせる検査 (T0811 の後続の候補)
- [ ] `just ci` が通る

## 参考

- docs/tasks/T0811-studio-third-party-notices.md・T0723-viewer-ui-and-i18n.md のメモ、scripts/check-licenses.mjs

## メモ
