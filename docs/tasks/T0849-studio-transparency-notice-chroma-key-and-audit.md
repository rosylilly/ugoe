---
id: T0849
title: "スタジオ: 透過が効かない注意に「背景をクロマキーにする」を付け、ui-audit に透過が効かない場面を足す"
status: in-progress
milestone: M10
crates: [apps/studio, tests, docs]
depends_on: [T0952]
assignee: claude-worker-T0849
---

# T0849: スタジオ: 透過が効かない注意に「背景をクロマキーにする」を付け、ui-audit に透過が効かない場面を足す

## 目的

T0846 で、出力ウィンドウの透過が効かない (X11 でコンポジタが無いなど) と分かるとき、「出力ウィンドウ」のパネルの「背景」の欄に注意と代わりの案を出すようにした。
残ったもの (T0846 の候補):

- 注意は案を文で言うだけで、その場で背景をクロマキーに替えられない。
- `tests/ui-audit` の偽のホストは透過を `supported` に固定していて、注意が出た画面を監査していない。

## 受け入れ条件

- [ ] 透過が効かない注意 (`Notice`) に「背景をクロマキーにする」のボタンがあり、押すと配信しているシーンの背景がクロマキーになる
      (シーンの背景の変更として保存され、取り消し方は既存の背景の欄と同じ)。押した後は注意が消える。部品のテストがある
- [ ] `tests/ui-audit` の偽のホストに透過が効かない様子の場面 (`-opaque` など) があり、`just ui-audit --only studio` の指摘が 0 件。tests/ui-audit/README.md の場面の表を直した
- [ ] docs/studio.md#出力ウィンドウの透過 と docs/manual/studio.md (困ったとき) を直し、画面が変わったところを T0919 のメモに書き足した
- [ ] `just ci` が通る

## 参考

- T0846 のメモ (調べた結果・やったこと・起票の候補)
- apps/studio/src/OutputPanel.svelte、apps/studio/src/lib/status.ts、tests/ui-audit/lib/studio-host.mjs

## メモ

- T0041 で起票した (元の候補: T0846 (クロマキーのボタン)・T0846 (ui-audit の場面))。T0952 (作業中) も tests/ui-audit を触るので、その後に着手する。
