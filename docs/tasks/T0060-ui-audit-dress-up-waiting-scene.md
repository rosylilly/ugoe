---
id: T0060
title: "ui-audit: エディタに「着せ替えで作り直しを待っている」場面 (待っている文の出る行) を足す"
status: done
milestone: M10
crates: [tests, docs]
depends_on: [T0055]
assignee: claude-worker-T0060
---

# T0060: ui-audit: エディタに「着せ替えで作り直しを待っている」場面 (待っている文の出る行) を足す

## 目的

T1666 で、プレビューの着せ替えは GPU を失っている間・WebPuppet の作り直しの途中に付け外しをすると、行のチェックが `wanted` を写し、
まだ届いていない行に「Puts this on / Takes this off when the preview is back.」の文が出るようになった。この行は `just ui-audit` のどの場面にも
写っておらず、コントラスト・はみ出し・押せる大きさを確かめていない。

## 受け入れ条件

- [x] `tests/ui-audit/lib/apps.mjs` のエディタに、着せ替えで作り直しを待っている行 (待っている文の出る行) を写す場面を足した
      (偽のホストで GPU を失った状態を作るなど、作り方をコメントに書く)。tests/ui-audit/README.md の場面の一覧に書いた
- [x] 場面に待っている文が写っていることを確かめた (撮った画像か、場面の中の検査で)
- [x] `just ui-audit --only editor` を流し、足した場面の指摘を直したか、意図したもの (理由) としてメモに書いた
- [x] `just ui-audit-check` と `just ci` が通る

## 参考

- T1666 のメモ (決めたこと・起票の候補)、apps/editor/src/lib/preview/dressUp.svelte.ts (`waiting`・`shown`・`pendingChange`)・DressUpSection.svelte
- tests/ui-audit/lib/apps.mjs、tests/ui-audit/README.md

## メモ

- T0058 で起票した (元の候補: T1666 (ui-audit に作り直しを待っている場面を足す))。
- 同じ tests/ui-audit を触る T0055 の後にした。
- **やったこと:** エディタの偽のホストのページの初期化 (`editorPageInit`) で WebGL2 のコンテキストを canvas ごとに覚え、
  `window.__uiAuditLoseGpu()` (描いている canvas のコンテキストを `WEBGL_lose_context` で失わせる)・`__uiAuditRestoreGpu()` を足した
  (apps/editor/browser-test の `PAGE_INIT`・`loseContext` と同じやり方)。`dressUpScenes` の `-preview-dress-up` の後に
  `-preview-dress-up-waiting` を足した: GPU を失わせ (ビューポートの「GPU の復元を待っています」を待つ)、押せる行のうち付いている
  最初の行 (CatEars) のチェックを外し、付いていない最初の行 (TwinTails。同じスロットの LongHair も外れる) にチェックを付けて撮る。
  「Puts this on…」「Takes this off…」の両方が出ていなければ場面の失敗にする。撮った後はコンテキストを戻し、作り直しで文が
  消えるのを待つ。GPU を失わせると wasm が `ugoe: WebGL context lost` をコンソールのエラーに出すので、`run.mjs` の `ctx` に
  `ignoreErrors(re)` (返した関数を呼ぶまで合うエラーを数えない) を足して、その間だけ数えないようにした。README の偽のホスト・
  場面の一覧・場面の足し方と、エディタの場面の数 (568) を直した。
- **確かめたこと:** `just ui-audit --only editor` (rebase 後にビルドから) で 568 場面。足した 12 場面 (13 の組のうち、開いた画面だけを撮る
  `ja-light-wide-dpr2` を除く) はすべて 0 件。全体はスクロールバーの 3 種 (T0055 のもの。`button.diamond`・
  `button.marker`・設定のダイアログの「Close」) だけで、足した場面には出ない。撮った画像 (`en-light-narrow`・`ja-light-forced-narrow`)
  で、行のチェックが付けたい組を写し、CatEars・LongHair に「Takes this off…」、TwinTails に「Puts this on…」が出ているのを見た。
  `just ui-audit-check`・`just ci` が通った。アプリの画面は変えていないので e2e は流していない。
- **場面で見えた気になる点 (直していない。アプリのコードで範囲の外):**
  - 読めないファイルの行 (`missing-ribbon`。`loaded: false`・`wanted: true`) にも、待っている間は「Puts this on when the preview is back.」が
    出て、押せないチェックが入って見える (待っていないときは外れて見える)。`pendingChange`・`shown` が `loaded` を見ないため。
    作り直しで読み込める `#pending` の行と、読めなかった行を分ける要がある。
  - 前に付けられなかった行 (TwinTails の「TwinTails was taken off: Slot "Hair" already has LongHair…」) の失敗の文が、待っている間に
    チェックを付け直しても残り、「Puts this on…」と並ぶ (`#want` が `problem` を消さない)。
- **起票の候補:**
  - エディタ: 着せ替えで待っている間、読めなかったモジュールの行に待っている文とチェックを出さない (上の 1 つ目)。
  - エディタ: 待っている間に付け直した行の前の失敗の文を消す (上の 2 つ目)。
