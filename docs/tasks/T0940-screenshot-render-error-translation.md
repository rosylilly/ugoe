---
id: T0940
title: "ビューアとエディタのスクリーンショットの失敗で RenderError を訳す"
status: todo
milestone: M9
crates: [apps/viewer, apps/editor, packages/player, docs]
depends_on: []
assignee:
---

# T0940: ビューアとエディタのスクリーンショットの失敗で RenderError を訳す

## 目的

T0465 で、wasm の `render`・`read_pixels` は GPU の喪失で `RenderError` (`code` = `device_lost`) を投げるようになり、
`@ugoe/player` に `describeRenderError` を足した。ところがスクリーンショットの保存の失敗は英語の文のまま出る (T0465 の候補):

- ビューア: `apps/viewer/src/App.svelte` の `saveScreenshot` (`t("screenshot.error", { message: errorText(e) })`)。
- エディタ: `apps/editor/src/lib/viewport/saveScreenshot.ts` の呼び出し側。

## 受け入れ条件

- [ ] ビューアのスクリーンショットの失敗が `RenderError` なら `describeRenderError` の文 (訳) を出す
- [ ] エディタのスクリーンショットの失敗が `RenderError` なら、エディタのカタログの文 (例: `preview.dressUp.problem.deviceLost` と同じ文) を出す
- [ ] 両方に部品か関数のテストがある。文言は日英のカタログにある
- [ ] `just ci` が通る

## 参考

- T0465 のメモ (起票の候補)
- bindings/web/src/render-errors.ts、packages/player/src/loadError.ts (`describeRenderError`)

## メモ

- T0036 で起票した (元の候補: T0465)。
