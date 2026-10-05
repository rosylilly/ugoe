---
id: T0940
title: "ビューアとエディタのスクリーンショットの失敗で RenderError を訳す"
status: done
milestone: M9
crates: [apps/viewer, apps/editor, packages/player, docs]
depends_on: []
assignee: claude-worker-T0940
---

# T0940: ビューアとエディタのスクリーンショットの失敗で RenderError を訳す

## 目的

T0465 で、wasm の `render`・`read_pixels` は GPU の喪失で `RenderError` (`code` = `device_lost`) を投げるようになり、
`@ugoe/player` に `describeRenderError` を足した。ところがスクリーンショットの保存の失敗は英語の文のまま出る (T0465 の候補):

- ビューア: `apps/viewer/src/App.svelte` の `saveScreenshot` (`t("screenshot.error", { message: errorText(e) })`)。
- エディタ: `apps/editor/src/lib/viewport/saveScreenshot.ts` の呼び出し側。

## 受け入れ条件

- [x] ビューアのスクリーンショットの失敗が `RenderError` なら `describeRenderError` の文 (訳) を出す
- [x] エディタのスクリーンショットの失敗が `RenderError` なら、エディタのカタログの文 (例: `preview.dressUp.problem.deviceLost` と同じ文) を出す
- [x] 両方に部品か関数のテストがある。文言は日英のカタログにある
- [x] `just ci` が通る

## 参考

- T0465 のメモ (起票の候補)
- bindings/web/src/render-errors.ts、packages/player/src/loadError.ts (`describeRenderError`)

## メモ

- T0036 で起票した (元の候補: T0465)。
- T0940 (claude-worker-T0940):
  - ビューア: `src/lib/screenshot.ts` に `screenshotErrorText` を足し、`RenderError` は `@ugoe/player` の `describeRenderError`、
    ほかは `errorText` にした。`App.svelte` の `saveScreenshot` がこれを `screenshot.error` に差し込む。テストは `screenshot.test.ts`。
  - エディタ: `src/lib/viewport/saveScreenshot.ts` に `screenshotErrorText` を足した。`device_lost` はエディタのカタログの
    `viewport.screenshot.deviceLost` (日英。文は `preview.dressUp.problem.deviceLost` と同じ。起動し直す案内がエディタ向け)、
    知らない `RenderError` のコードは `describeRenderError` (例外の文)、ほかは `errorText`。`Viewport.svelte` がこれを使う。
    テストは `viewport/screenshot.test.ts`。
  - 文書: docs/editor.md#スクリーンショット と apps/viewer/README.md#画像の保存-t0420 に 1 文ずつ足した。packages/player は触っていない。
  - 実際の GPU の喪失での確かめはしていない (関数のテストで `RenderError` を模した)。
  - 起票の候補: エディタのビューポートの読み込みの失敗 (controller の `describeLoadError`) は `device_lost` でプレイヤーのカタログの
    「アプリを起動し直して」の文になる。エディタ向けの文 (エディタを起動し直す) に揃えるか。
