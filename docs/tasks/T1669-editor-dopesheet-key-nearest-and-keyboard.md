---
id: T1669
title: "エディタ: ドープシートのキーの印で、押せる範囲が重なる所は近いキーをつかみ、Enter・Space でもキーを選べるようにする"
status: done
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

- [x] ドープシートのキーの印で、押せる範囲が重なる所は押した所に近いキーをつかむ (`nearestMark` を使う)。部品のテストで、直す前に落ちることも確かめた
- [x] キーの印を Enter・Space で押すとキーが選ばれる (ポインタと同じ結果。ドラッグはポインタだけでよい)。部品のテスト
- [x] docs/editor.md (ドープシートの操作・代わりの手段の表) と docs/ui.md (近い印の重なり) を直した。マニュアルの更新の候補をメモに書いた
- [x] `just ui-audit --only editor --variants <必要な組>` の指摘が増えない。`just e2e --only editor` と `just ci` が通る

## 参考

- T1668 のメモ、T1634 (`nearestMarkerKey`)、apps/editor/src/lib/panels/timeline/

## メモ

- T1668 のワーカーの報告から起票した (2026-10-07)。T1666 が MotionEditor を並行して触っていたので、その後に着手する。
- やったこと:
  - `MotionEditor.svelte` の `onKeyDown` で、押した所 (レーンの中の横の位置) に近いキーを `nearestMark` で選んでつかむようにした
    (行の印は縦に並ばないので y はすべて 0。押せる範囲は `.diamond` と同じ 24px。どの範囲にも入らなければ押した印)。
    選択と再生位置の移動を `pickKey` にまとめ、印のボタンの `onkeydown` で Enter・Space でも同じくキーを選ぶ
    (`preventDefault` と `stopPropagation` で、パネルの Space の再生・停止にしない。ドラッグはポインタだけ)。
  - テスト (`MotionEditor.test.ts`): 幅 44px で 0 秒と 1 秒のキーの押せる範囲が重なり、上の 1 秒の印の 0 秒に近い所を押すと
    0 秒のキーが選ばれ、ドラッグも 0 秒のキーを動かす。Enter・Space で印を押すとキーが選ばれて再生位置が移り、再生は始まらない。
    実装の前に流して、3 つとも落ちる (近い方は 1 秒が選ばれ、キーボードは選ばれない) ことを確かめた。
  - docs/editor.md (ドープシートの行・キーボードの行) と docs/ui.md (近い印の重なり) を直した。
  - `just ui-audit --only editor --variants zoom200,narrow` (322 場面) を、直す前の MotionEditor.svelte を作業ツリーに一時的に
    戻して撮った基準と `--compare` で比べた: +0 -0 ~0 (すべて 0 件)。`just e2e --only editor` (16) と `just ci` が通った。
- やらなかったこと: ドープシートの印に `data-ui-audit-overlay` は付けていない (`position: absolute` で重なりの検査にもとから出ない)。
- マニュアルの更新の候補: docs/manual/motions.md の操作の表「行のキーの印を押す」に、Enter・Space でも選べることと、
  キーが近いときは押した所に近いキーを選ぶことを書き足す (tools.md の「キーの印のドラッグ」も同様)。
