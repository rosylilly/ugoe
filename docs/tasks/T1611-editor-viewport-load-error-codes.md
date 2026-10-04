---
id: T1611
title: "エディタ: ビューポートとプレビューの読み込みの失敗を wasm の LoadError のコードから訳す"
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee:
---

# T1611: エディタ: ビューポートとプレビューの読み込みの失敗を wasm の LoadError のコードから訳す

## 目的

T0434 で wasm の読み込みの失敗を `LoadError` (`code` と補足の値) にしたが、エディタのビューポート (`apps/editor/src/lib/viewport/controller.svelte.ts` の
`load_json`) は英語の文を出す (T0434 の候補)。`@ugoe/player` の `describeLoadError` と同じコードで訳す。

## 受け入れ条件

- [ ] ビューポート (とプレビュー) の読み込みの失敗が、`code` と補足の値から今の言語の文になる。知らないコードは英語の文のまま
- [ ] 訳の表 (`en.json`・`ja.json`) に `codes.json` の読み込みのエラーのすべてのコードがあり、テストがある
- [ ] `just i18n` と `just ci` が通る

## 参考

- T0434 のメモ (起票の候補)、packages/player/src/loadError.ts、crates/ugoe-wasm/codes.json

## メモ

- T0026 で起票した (元の候補: T0434 のエディタのビューポート)。訳を `@ugoe/player` と共有できるなら共有する。
