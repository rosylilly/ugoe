---
id: T0901
title: i18n: 翻訳の共有パッケージ @ugoe/i18n
status: todo
milestone: M8
crates: [packages, scripts, justfile, docs]
depends_on: [T0900]
assignee:
---

# T0901: i18n: 翻訳の共有パッケージ @ugoe/i18n

## 目的

3 つのアプリと共有パッケージが同じ仕組みで日本語と英語を出せるようにする。

## 受け入れ条件

- [ ] packages/i18n: 言語ごとのカタログ (型でキーの過不足を検査)、引数の差し込み、複数形、数値・日付の書式 (Intl)、言語の検出 (OS / ブラウザ) と上書き、
      切り替えたときの再描画 (Svelte 5 の runes)
- [ ] packages/face-tracking の strings.ts をこの仕組みに移す (表示は変えない)
- [ ] カタログのキーの過不足と、未使用のキーを検査する `just i18n` (just ci に入れる)
- [ ] Vitest のテストと README。`just ci`・`just licenses` が通る

## 参考

- ADR 0029、docs/ui.md

## メモ
