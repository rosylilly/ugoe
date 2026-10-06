---
id: T0065
title: "ui-audit: エディタのツリーのドラッグの途中 (ドロップの印) を撮る場面を足す"
status: in-progress
milestone: M10
crates: [tests, docs]
depends_on: [T1675, T0064]
assignee: claude-worker-T0065
---

# T0065: ui-audit: エディタのツリーのドラッグの途中 (ドロップの印) を撮る場面を足す

## 目的

T1670 でツリーの行のドロップの印 (`drop-into`・`drop-above`・`drop-below`・`drop-bad`) を強制色でも描くようにしたが、ドラッグの途中を写す
`just ui-audit` の場面が無く、T1670 では一時的なページで確かめただけだった。コントラスト・強制色で印が見えるかを監査で見続けるために場面を足す。

## 受け入れ条件

- [ ] `tests/ui-audit/lib/apps.mjs` のエディタに、ツリーの行をポインタで押して動かし、離さずに撮る場面を足した (TreePanel.svelte の
      ポインタのドラッグ。少なくとも中へ (`drop-into`)・落とせない所 (`drop-bad`) の 2 つ。手前・奥も撮るかは場面の数との兼ね合いで決める)。
      撮った後にドラッグを取り消し (Esc や `pointercancel`)、後の場面に残さない。作り方をコメントに書き、tests/ui-audit/README.md の場面の一覧に書いた
- [ ] 場面にドロップの印が写っていることを確かめた (撮った画像か、場面の中で `.row.drop-*` を待つ検査で)。強制色の組の画像も見た
- [ ] `just ui-audit --only editor` を流し、足した場面の指摘を直したか、意図したもの (理由) としてメモに書いた
      (直すのがアプリのコードなら、範囲の外なので起票の候補にする)
- [ ] `just ui-audit-check` と `just ci` が通る

## 参考

- T1670 のメモ (強制色のドロップの印・起票の候補)、apps/editor/src/lib/panels/TreePanel.svelte (`onRowPointerDown`・`onPointerMove`・`cancelDrag`)、
  apps/editor/src/lib/panels/tree/TreeItem.svelte (`.row.drop-*`)
- tests/ui-audit/lib/apps.mjs、tests/ui-audit/README.md

## メモ

- T0063 で起票した (元の候補: T1670 (ドラッグの途中を撮る ui-audit の場面が無い))。
- 同じツリーの行の強制色の CSS を直す T1675 と、同じ tests/ui-audit を触る T0064 の後にした。
