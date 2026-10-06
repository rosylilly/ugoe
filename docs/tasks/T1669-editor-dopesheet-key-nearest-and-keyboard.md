---
id: T1669
title: "エディタ: ドープシートのキーの印で、押せる範囲が重なる所は近いキーをつかみ、Enter・Space でもキーを選べるようにする"
status: in-progress
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1669
---

# T1669: エディタ: ドープシートのキーの印で、押せる範囲が重なる所は近いキーをつかみ、Enter・Space でもキーを選べるようにする

## 目的

T1668 でモーションのグラフの印は、押せる範囲が重なる所では押した所に近い印をつかむようにした (`timeline.ts` の `nearestMark`)。
ドープシートのキーの印 (`MotionEditor.svelte` の `.diamond`) も近いと押せる範囲が重なり、後に描かれたキーが取る。`position: absolute` なので
ui-audit の重なりの検査には出ない。また、この印はボタンだが `onpointerdown` だけでキーを選ぶので、キーボード (Enter・Space) で押しても選ばれない (T1668 の候補 2 つ)。

## 受け入れ条件

- [ ] ドープシートのキーの印で、押せる範囲が重なる所は押した所に近いキーをつかむ (`nearestMark` を使う)。部品のテストで、直す前に落ちることも確かめた
- [ ] キーの印を Enter・Space で押すとキーが選ばれる (ポインタと同じ結果。ドラッグはポインタだけでよい)。部品のテスト
- [ ] docs/editor.md (ドープシートの操作・代わりの手段の表) と docs/ui.md (近い印の重なり) を直した。マニュアルの更新の候補をメモに書いた
- [ ] `just ui-audit --only editor --variants <必要な組>` の指摘が増えない。`just e2e --only editor` と `just ci` が通る

## 参考

- T1668 のメモ、T1634 (`nearestMarkerKey`)、apps/editor/src/lib/panels/timeline/

## メモ

- T1668 のワーカーの報告から起票した (2026-10-07)。T1666 が MotionEditor を並行して触っていたので、その後に着手する。
