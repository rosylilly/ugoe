---
id: T1667
title: "エディタ: T1654 の後、200% の拡大でツリーの見えない「本体」の札の文が左の列の外へはみ出す (ui-audit の escapes)"
status: done
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

- [x] 見せずに読ませる「本体」の文の位置の基準を行 (か札) の中に置き、`escapes` が出ないようにした。読み上げ (「base hidden」) と見た目は変えない
- [x] `just ui-audit --only editor --variants <zoom200 を含む組>` で、上の場面の `escapes` が 0 件。T1654 の場面 (`-tree-hidden-hover`・`-tree-base-hover`) のほかの検査も増えない (`--compare`)
- [x] `TreeItem.test.ts` が通る。`just e2e --only editor` と `just ci` が通る

## 参考

- T1654・T0954・T1657、docs/ui.md (送り)、apps/editor のツリー (TreeItem.svelte)

## メモ

- T0053 のワーカーの報告から起票した (2026-10-07)。
- やったこと (2026-10-07): 本体の札をまとめた札 (`.ed-tag.base-merged`、`TreeItem.svelte`) に `position: relative` を付け、
  中の見せない「本体」の文字の位置の基準を札にした。札の見た目と読み上げ (「base hidden」)・`title` は変えていない。
  docs/ui.md の「送り」(アプリが置く見せずに読ませる文字の例に札を足した) と「エディタのツリーの札と選んだ行 (T1654)」に書いた。
- 確かめたこと: `just ui-audit --only editor --variants zoom200` (92 場面) を直す前 (`--out target/ui-audit-before`) と後で比べて
  +0 -1 ~0 (`span.ugoe-visually-hidden escapes aside.left` の 6 場面が消え、すべての検査で 0 件)。T1654 の組
  (`en-dark-wide,ja-dark-narrow,en-light-wide,ja-light-narrow,forced`、278 場面) も直した後ですべての検査が 0 件
  (`-tree-hidden-hover`・`-tree-base-hover` を含む)。`TreeItem.test.ts` (7 件)・`just e2e --only editor` (16 件)・`just ci` が通った。
- やらなかったこと: 位置の基準の単体テストは足していない (happy-dom は `offsetParent` もレイアウトも計算しないため。
  検出は ui-audit の `escapes` が担う。T1657 と同じ)。
- 起票の候補: なし。
