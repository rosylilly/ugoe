---
id: T0921
title: アプリの自前の .visually-hidden を @ugoe/ui の .ugoe-visually-hidden に置き換える
status: in-progress
milestone: M9
crates: [apps/editor, apps/viewer, docs]
depends_on: [T0912]
assignee: claude-worker-T0921
---

# T0921: アプリの自前の .visually-hidden を @ugoe/ui の .ugoe-visually-hidden に置き換える

## 目的

`@ugoe/ui` の `tokens.css` には、見せずに読み上げだけさせる文字のクラス `.ugoe-visually-hidden` がある (T0912 で公開のものとして
docs/ui.md#そのほか と packages/ui/README.md に書いた)。ところが、エディタとビューアのいくつかの部品は、同じ指定を自前の
`.visually-hidden` として `<style>` に書き直している。これを `.ugoe-visually-hidden` に置き換え、自前の CSS を消す。

2026-10-04 の時点の場所:

- `apps/viewer/src/lib/MemoryDialog.svelte`
- `apps/editor/src/lib/panels/timeline/MotionEditor.svelte`
- `apps/editor/src/lib/panels/timeline/CurveGraph.svelte`
- `apps/editor/src/lib/ui/TabStrip.svelte`

(`apps/editor/src/lib/panels/slots/SlotsPanel.svelte` はすでに `.ugoe-visually-hidden` を使っている。)

## 受け入れ条件

- [ ] 上の部品の `.visually-hidden` を `.ugoe-visually-hidden` にし、`<style>` の自前の指定を消した (`rg "visually-hidden" apps` で `ugoe-` の付かないものが無い)
- [ ] 読み上げの名前を確かめるテスト (あれば) が通る
- [ ] `just ci` が通る

## 参考

- docs/ui.md#そのほか
- T0912 のメモ

## メモ
