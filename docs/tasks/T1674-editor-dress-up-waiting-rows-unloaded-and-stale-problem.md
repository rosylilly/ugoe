---
id: T1674
title: "エディタ: 着せ替えで GPU を待っている間、読めなかった行に待っている文が出ることと、付け直した行に前の失敗の文が残ること"
status: in-progress
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1674
---

# T1674: エディタ: 着せ替えで GPU を待っている間、読めなかった行に待っている文が出ることと、付け直した行に前の失敗の文が残ること

## 目的

T0060 の ui-audit の場面 `-preview-dress-up-waiting` で、T1666 の待っている間の行の表示に 2 つの誤りが見えた (T0060 の候補)。

1. 読めないファイルの行 (`missing-ribbon`) にも、待っている間は「Puts this on when the preview is back.」が出て、押せないチェックが入って見える。
   `pendingChange` と `shown` が、読み込めたか (`loaded`) を見ていないため。
2. 前に付けられなかった行の失敗の文 (例: TwinTails の「Slot "Hair" already has LongHair…」) が、待っている間にチェックを付け直しても消えず、
   「Puts this on…」と並んで出る。`#want` が `problem` を消さないため。

## 受け入れ条件

- [ ] 読めなかった行には、待っている間も待っている文を出さず、チェックも付けたい組を写さない (押せないまま、前の状態)
- [ ] 待っている間に付け直した行は前の失敗の文を消す (作り直した後に付けられなければ、その時の理由をまた出す)
- [ ] 部品のテスト (`dressUp.test.ts`・`DressUpSection.test.ts`) で、どちらも直す前に落ちることを確かめた
- [ ] `just ui-audit --only editor --variants <必要な組>` の `-preview-dress-up-waiting` で、上の 2 つが消えたことを画像で確かめた。`just e2e --only editor`・`just editor-browser`・`just ci` が通る

## 参考

- T1666・T0060、apps/editor/src/lib/preview/dressUp.svelte.ts・DressUpSection.svelte

## メモ

- T0060 のワーカーの報告から起票した (2026-10-07)。
