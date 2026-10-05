---
id: T0941
title: "@ugoe/player とエディタで CreateError を訳し、エディタのビューポートの device_lost をエディタ向けの文にする"
status: in-progress
milestone: M9
crates: [packages/player, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0941
---

# T0941: @ugoe/player とエディタで CreateError を訳し、エディタのビューポートの device_lost をエディタ向けの文にする

## 目的

- T0475 で、wasm の `WebPuppet.create` は GPU・ブラウザの都合の失敗を `CreateError` (`backend_unavailable`・`context_unavailable`・
  `adapter_unavailable`・`device_unavailable`) で投げるようになったが、`Player` は `errors.renderStart` に英語の `message` を埋めて出し、
  エディタのビューポートは `errorText(e)` (英語) を出す (T0475 の候補。T0940 の `RenderError` と同じ形で訳す)。
- エディタのビューポートの読み込みの失敗 (`ViewportController` の `describeLoadError`) は、`device_lost` で `@ugoe/player` のカタログの
  「アプリを起動し直して」の文になる。エディタ向けの文 (エディタを起動し直す) に揃える (T0940 の候補)。

## 受け入れ条件

- [ ] `@ugoe/player` に `CreateError` の訳 (例: `describeCreateError`) を足し、`Player` の作成の失敗が `CreateError` なら訳した文を出す
      (`context_unavailable` なら「ほかのタブを閉じるか、少し待って作り直してください」のような文)。Vitest で確かめる
- [ ] エディタのビューポートの作成の失敗が `CreateError` なら、エディタのカタログの文を出す。読み込みの失敗の `device_lost` もエディタのカタログの文
      (`viewport.screenshot.deviceLost` と同じ文など) を出す。関数か部品のテストがある
- [ ] 文言は日英のカタログにある。docs/ui.md#webpuppet-の寿命 (または docs/editor.md) に、どの文がどこから来るかを書いた
- [ ] `just ci` が通る

## 参考

- T0475・T0940 のメモ (起票の候補)
- bindings/web/src/create-errors.ts、packages/player/src/loadError.ts (`describeRenderError`・`describeLoadError`)、apps/editor/src/lib/viewport/controller.svelte.ts・saveScreenshot.ts

## メモ

- T0038 で起票した (元の候補: T0475 (`CreateError` の訳)・T0940 (エディタの `device_lost`))。
