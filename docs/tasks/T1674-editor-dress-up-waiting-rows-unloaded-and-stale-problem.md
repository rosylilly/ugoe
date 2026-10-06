---
id: T1674
title: "エディタ: 着せ替えで GPU を待っている間、読めなかった行に待っている文が出ることと、付け直した行に前の失敗の文が残ること"
status: done
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

- [x] 読めなかった行には、待っている間も待っている文を出さず、チェックも付けたい組を写さない (押せないまま、前の状態)
- [x] 待っている間に付け直した行は前の失敗の文を消す (作り直した後に付けられなければ、その時の理由をまた出す)
- [x] 部品のテスト (`dressUp.test.ts`・`DressUpSection.test.ts`) で、どちらも直す前に落ちることを確かめた
- [x] `just ui-audit --only editor --variants <必要な組>` の `-preview-dress-up-waiting` で、上の 2 つが消えたことを画像で確かめた。`just e2e --only editor`・`just editor-browser`・`just ci` が通る

## 参考

- T1666・T0060、apps/editor/src/lib/preview/dressUp.svelte.ts・DressUpSection.svelte

## メモ

- T0060 のワーカーの報告から起票した (2026-10-07)。
- **やったこと:** `DressUp` に `#willLoad(entry)` (作り直した WebPuppet に `loaded` で読み込む行か: 読み込んである・`#bytes` にある・
  `#pending` で待っている。今の編集と同じ名前のファイルの行は読み込まない) を足した。`shown` は待っている間でも読み込まない行には
  `attached` を出す (`pendingChange` も `null`)。`#want` は読み込む行の `problem` を消す (読めない理由は残す)。行が `#pending` で
  待っているかを画面が追えるよう、`#pending` を `$state.raw` にして書き換えを代入にした。docs/editor.md の「待っている間の行」と
  dressUp.svelte.ts・DressUpSection.svelte の頭のコメントを直した。
- **確かめたこと:** 部品のテスト (`dressUp.test.ts` に 3 つ・`DressUpSection.test.ts` に 2 つ) を足し、読めない行と付け直した行の 4 つが
  直す前に落ちることを見た (作り直しの途中に読み直した行が「Puts this on…」を出すテストは直す前から通る。読み込む行を外さないための
  もの)。`just ui-audit --only editor --variants en-light-narrow` (47 場面) の `-preview-dress-up-waiting` の画像で、`missing-ribbon` が
  外れたチェックで文が無く、TwinTails に前の失敗の文が無いのを見た。指摘はスクロールバーの既知の 2 種 (T0055) だけで、この場面には
  出ない。`just e2e --only editor` は 1 回目に見本を開く段の知らせの待ちが時間切れ (着せ替えと無関係。ロードアベレージ 16) で、2 回目に
  通った。`just editor-browser` (4 場面)・rebase 後の `just ci` が通った。
- **起票の候補:** なし。
