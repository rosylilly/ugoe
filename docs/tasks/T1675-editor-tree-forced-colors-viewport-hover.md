---
id: T1675
title: "エディタ: 強制色で、ツリーのビューポートからのホバー (hovered・hovered-within) が見えないのを直す"
status: in-progress
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1675
---

# T1675: エディタ: 強制色で、ツリーのビューポートからのホバー (hovered・hovered-within) が見えないのを直す

## 目的

T1670 で強制色のツリーの行の枠・選択・ドロップの印を描き直したが、ホバー (`.row:hover`・`.row.hovered` の `background-image` の重ね) と、
畳んだ行の中のメッシュに乗っている印 (`.row.hovered-within` の左端の内側の `box-shadow`) は強制色で消える。行にポインタがあるホバーは
ポインタの位置で分かるが、ビューポートのメッシュに乗ったときのツリーの対応 (`hovered`・`hovered-within`) は行にポインタが無いので、
強制色では分からない (docs/editor.md#選択とホバーの見せ方)。

## 受け入れ条件

- [ ] 強制色で `hovered` と `hovered-within` を見分けられる形 (枠の左辺の線・`Highlight` の枠など、システムの色だけで) を決め、
      `@media (forced-colors: active)` で描いた。選択・キーボードの位置・本体の行・ドロップの印 (T1670) と重なったときにも見分けられる。
      行にポインタがあるホバー (`:hover`) も描くかを決め、理由をメモと docs/ui.md の「エディタのツリーの行の札と強制色」に書いた
- [ ] 強制色でないときの見た目は変わらない
- [ ] `just ui-audit --only editor --variants forced` (と強制色でない組を 1 つ) を直す前と `--compare` で比べ、`-tree-hover`・
      `-tree-hover-collapsed` の画像で印が見えることを確かめた
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T1670 のメモ (強制色の行・起票の候補)、apps/editor/src/lib/panels/tree/TreeItem.svelte (`.row.hovered`・`.row.hovered-within`・強制色の節)
- docs/ui.md (強制色の節)、docs/editor.md#選択とホバーの見せ方
- tests/ui-audit/lib/apps.mjs の `tree-hover`・`tree-hover-collapsed` の場面

## メモ

- T0063 で起票した (元の候補: T1670 (強制色でツリーのホバーと畳んだ行の中のメッシュの印が出ない))。
- 本物の Windows の高コントラストと WebView2 では見られないなら、Chrome の強制色のエミュレーションで確かめた範囲をメモに書く。
