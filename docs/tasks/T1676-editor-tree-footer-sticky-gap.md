---
id: T1676
title: "エディタ: ツリーの下の欄 (sticky) が左の列の余白の分だけ下端から浮き、その下に木の行が覗く"
status: in-progress
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1676
---

# T1676: エディタ: ツリーの下の欄 (sticky) が左の列の余白の分だけ下端から浮き、その下に木の行が覗く

## 目的

ツリーの下の欄 (`TreePanel.svelte` の `.footer`、`position: sticky; bottom: 0`) が、左の列の内側の余白の分だけ下端から浮いている。
狭い幅で理由の文 (`.hint.error`) が出ると、その下の 12px ほどに木の行が覗く (ui-audit の `en-light-narrow-tree-drop-bad` の画像。T0065 の候補)。
監査の検査では出ない。

## 受け入れ条件

- [ ] 下の欄を左の列の下端に付け、下に木の行が覗かないようにした (`.head` と同じく `bottom` を負の余白にするなど)。見出し (`.head`) の扱いとそろえた
- [ ] `just ui-audit --only editor --variants <狭い幅を含む組>` で、`-tree-drop-bad` などの画像で直ったことを見た。指摘は増えない (`--compare`)
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0065 のメモ、apps/editor/src/lib/panels/tree/TreePanel.svelte

## メモ

- T0065 のワーカーの報告から起票した (2026-10-07)。
