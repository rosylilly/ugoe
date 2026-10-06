---
id: T1667
title: "エディタ: T1654 の後、200% の拡大でツリーの見えない「本体」の札の文が左の列の外へはみ出す (ui-audit の escapes)"
status: in-progress
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1667
---

# T1667: エディタ: T1654 の後、200% の拡大でツリーの見えない「本体」の札の文が左の列の外へはみ出す (ui-audit の escapes)

## 目的

T1654 で、札が 2 つ並ぶ行では「本体」の札を見せずに置くようにした (`span.ugoe-visually-hidden "base"/"本体"`)。T0053 のワーカーが
`just ui-audit --only editor` を全組で流すと、`*-zoom200` の `-base-change`・`-tree-base-hover`・`-module-ai` で、この文が
`escapes aside.left` (送る枠の外を位置の基準にしてはみ出す。T0954 の検査) と出た。T1654 の前には出ていなかった。
見せずに読ませる文の位置の基準が行の中に無いため (T0954・T1657 と同じ形)。

## 受け入れ条件

- [ ] 見せずに読ませる「本体」の文の位置の基準を行 (か札) の中に置き、`escapes` が出ないようにした。読み上げ (「base hidden」) と見た目は変えない
- [ ] `just ui-audit --only editor --variants <zoom200 を含む組>` で、上の場面の `escapes` が 0 件。T1654 の場面 (`-tree-hidden-hover`・`-tree-base-hover`) のほかの検査も増えない (`--compare`)
- [ ] `TreeItem.test.ts` が通る。`just e2e --only editor` と `just ci` が通る

## 参考

- T1654・T0954・T1657、docs/ui.md (送り)、apps/editor のツリー (TreeItem.svelte)

## メモ

- T0053 のワーカーの報告から起票した (2026-10-07)。
