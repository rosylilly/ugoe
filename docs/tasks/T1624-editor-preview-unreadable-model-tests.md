---
id: T1624
title: "エディタ: プレビュー (PreviewPanel・DifferenceOverlay) を読めないモデルで描く部品のテスト"
status: in-progress
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1624
---

# T1624: エディタ: プレビュー (PreviewPanel・DifferenceOverlay) を読めないモデルで描く部品のテスト

## 目的

T1613 で、ビューポートの重ねものとパネルを読めないモデル (`doc.model.json` が壊れている) で描く部品のテストを足した
(`viewport/unreadableModel.test.ts`・`panels/unreadableModel.test.ts`)。プレビュー (`apps/editor/src/lib/preview/PreviewPanel.svelte`・
`DifferenceOverlay.svelte`) は wasm と `PreviewMode` が要るので対象にしなかった (T1613 の候補)。

## 受け入れ条件

- [ ] `PreviewPanel`・`DifferenceOverlay` を、最初から読めないモデルと、読めるモデルで描いた後で読めなくなるモデルで描くテストがある (wasm と `PreviewMode` は偽物)。例外を出さず、何も送らない
- [ ] テストの道具 (`harness.svelte.ts`) に足したものを docs/testing.md#エディタの部品のテスト の「読めないモデル」の行に書いた
- [ ] 見つかった不具合は直した (テストが先に落ちることを確かめた)
- [ ] `just ci` が通る

## 参考

- T1613 のメモ (起票の候補)
- docs/testing.md#エディタの部品のテスト

## メモ

- T0036 で起票した (元の候補: T1613)。
