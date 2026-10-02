---
id: T0908
title: i18n: 画面の直書きの文言を見つける検査
status: todo
milestone: M8
crates: [scripts, justfile, docs]
depends_on: [T0901]
assignee:
---

# T0908: i18n: 画面の直書きの文言を見つける検査

## 目的

翻訳を通さない文言が入り込むのを防ぐ。

## 受け入れ条件

- [ ] .svelte と画面の .ts の中で、カタログを通さない表示用の文言 (テキストノード・title・aria-label・placeholder など) を見つける検査。
      移行の途中は、アプリごとに許す一覧 (移行が済んだら空にする) を持てる
- [ ] `just i18n` に入れ、just ci で動かす
- [ ] `just ci` が通る

## 参考

- T0901

## メモ
