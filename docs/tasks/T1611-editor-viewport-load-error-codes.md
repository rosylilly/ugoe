---
id: T1611
title: "エディタ: ビューポートとプレビューの読み込みの失敗を wasm の LoadError のコードから訳す"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1611
---

# T1611: エディタ: ビューポートとプレビューの読み込みの失敗を wasm の LoadError のコードから訳す

## 目的

T0434 で wasm の読み込みの失敗を `LoadError` (`code` と補足の値) にしたが、エディタのビューポート (`apps/editor/src/lib/viewport/controller.svelte.ts` の
`load_json`) は英語の文を出す (T0434 の候補)。`@ugoe/player` の `describeLoadError` と同じコードで訳す。

## 受け入れ条件

- [x] ビューポート (とプレビュー) の読み込みの失敗が、`code` と補足の値から今の言語の文になる。知らないコードは英語の文のまま
- [x] 訳の表 (`en.json`・`ja.json`) に `codes.json` の読み込みのエラーのすべてのコードがあり、テストがある
- [x] `just i18n` と `just ci` が通る

## 参考

- T0434 のメモ (起票の候補)、packages/player/src/loadError.ts、crates/ugoe-wasm/codes.json

## メモ

- T0026 で起票した (元の候補: T0434 のエディタのビューポート)。訳を `@ugoe/player` と共有できるなら共有する。
- 2026-10-05 (claude-worker-T1611): 完了。
  - やったこと: `ViewportController.#load` の失敗を `@ugoe/player` の `describeLoadError` で訳す (エディタは既に @ugoe/player に
    依存しているので、訳はプレイヤーのカタログ (名前空間 `player` の `loadError`) をそのまま共有し、エディタのカタログには足していない)。
    プレビューモードも同じ `#load` を通るので両方に効く。描画 (`render`) の失敗とその他は今までどおり `errorText`。
  - テスト: `controller.test.ts` (unsupported_version が日本語の文になる・知らないコードは wasm の文のまま。修正の前は失敗するのを確かめた)、
    `viewport/loadError.test.ts` (codes.json の `load_error` のすべてのコードと `image`・`limit` の値の組が、すべての言語で文になり、
    `{{` が残らず、英語以外は原文と違う文になる)。
  - docs/editor.md#ビューポート に 1 項目足した。
  - `just i18n`・`just ci` は通った。
  - 起票の候補: なし。
