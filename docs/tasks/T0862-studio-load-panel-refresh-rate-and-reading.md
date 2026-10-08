---
id: T0862
title: "スタジオ: 「フレームレートと負荷」の受け手ごとの行に描く機会の回数 (画面のリフレッシュレート) を出し、スクリーンリーダーの読み上げの区切りを直す"
status: done
milestone: M8
crates: [apps/studio, docs, tests/ui-audit]
depends_on: []
assignee: claude-worker-T0862
---

# T0862: スタジオ: 受け手ごとの行に描く機会の回数を出し、読み上げを直す

## 目的

受け手 (出力ウィンドウ・ブラウザソース) は変わったときだけ描く (T0857) ので、「フレームレートと負荷」の描画の回数は届いたフレームの数に従い、
刻みの速さより少なくなることがある。注意の判断は描く機会 (`requestAnimationFrame`) の回数 (`refreshFps`。報告にはもう載っている) で行うが、
画面には出していないので、利用者は「描いた回数が少ないのは画面のリフレッシュレートのせいか」を見分けられない。受け手ごとの行に
描く機会の回数も出す。

あわせて、受け手ごとの行 (`<dd>`) の中の値の文と、2 つ目の行の API と GPU の名前 (T0860) が、スクリーンリーダーでは区切り無しに
続けて読まれる (T0860 のメモ)。行の値の読ませ方をそろえて見直す。

## 受け入れ条件

- [x] 出力ウィンドウ・ブラウザソースの行に、描いた回数と並べて描く機会の回数 (`refreshFps`) を出す (例: 「描画 42 回/秒 (画面 144 回/秒)」)。
      `refreshFps` の無い受け手 (仮想カメラ) と古い報告 (`stale`) では出さない。文言は @ugoe/i18n の日英に足した
- [x] 受け手ごとの行の値 (描画の回数・描く機会・1 回の時間・捨てたフレーム・API と GPU の名前) が、スクリーンリーダーで区切られて読まれる
      (例: 値ごとに要素を分けて句読点か `aria-label` で区切る。どの形にしたかを LoadPanel.svelte の頭のコメントに書く)。
      LoadPanel.svelte.test.ts で、読み上げの名前 (アクセシブルな名前か text content) に区切りがあることを確かめた
- [x] 注意の判断 (`apps/studio/src/lib/load.ts`) は変えない (表示だけ)
- [x] docs/studio.md#負荷の表示 の表に描く機会の回数の表示を書いた。docs/manual/studio.md の「フレームレートと負荷」の説明も直した
      (スクリーンショット docs/manual/images/studio-load.png は撮り直せたら撮り直す)
- [x] 画面の並び・文言を変えたので `just e2e --only studio` と `just ui-audit --only studio` を流した
- [x] `just ci` が通る

## 参考

- T0857 のメモ (起票の候補)、T0860 のメモ (起票の候補。読み上げ)
- apps/studio/src/LoadPanel.svelte・LoadPanel.svelte.test.ts・lib/load.ts・lib/host.ts (`RendererLoad`)、docs/studio.md#負荷の表示
- docs/ui.md (部品と読み上げ)

## メモ

- T0077 で起票した (元の候補: T0857 (描く機会の回数を出す)・T0860 (読み上げの区切り))。同じ行の表示なので 1 つにした。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T0863 (トラッキングの待ちを出す) も同じパネルを触るので、T0863 はこのタスクの後にする。
- **やったこと:**
  - 出力ウィンドウ・ブラウザソースの行を「描画 42 回/秒 (画面 144 回/秒)、1 回 1.5 ms (最大 3 ms)、捨てたフレーム 0」の形にした
    (`load.drawnRefresh`)。`refreshFps` が `null` の受け手 (仮想カメラ) は「描画 30 回/秒」(`load.drawn`)、古い報告は「描いていません」だけ。
    文言は日英 (`load.drawn`・`drawnRefresh`・`renderTime`・`dropped`・`separator`)。1 つの文だった `load.rendererValue` と `load.rendererGpu` は消した。
  - 読み上げの区切り: 値を見える区切りの文字 (`load.separator`。日本語「、」・英語 ", ") でつなぐ。前は日本語で「・」を使っていて、読み上げで
    黙って飛ばされることがあった。2 行目 (API と GPU の名前。API と GPU の間も同じ区切り) の前は見た目では改行なので、区切りを
    `.ugoe-visually-hidden` で読み上げにだけ置いた (`aria-label` は `<dd>` の名前にならないので使わない)。LoadPanel.svelte の頭のコメントと
    docs/studio.md#負荷の表示 に書いた。隠した区切りは `position: absolute` なので、`.load__gpu` を `position: relative` にして行の中に留めた
    (付けないと `just ui-audit` の横のはみ出しに出た)。
  - テスト: LoadPanel.svelte.test.ts に 3 つ (行の text content が値を区切りでつないだものになる・隠した区切りがある、仮想カメラと古い報告に
    描く機会を出さない、API が無ければ 2 行目を出さない)。`load.ts` は変えていない。
  - 文書: docs/studio.md#負荷の表示 (表の行と読み上げの区切り)、docs/manual/studio.md (描く機会の回数の説明と画像の説明)。
    マニュアルの画像 studio-load.png を `just e2e --manual --only studio` で撮り直し、その 1 枚だけを run.mjs と同じ `convert -strip -dither None -colors 256`
    で写した (「描画 60 回/秒 (画面 60 回/秒)、1 回 6.15 ms (最大 10 ms)、捨てたフレーム 0」「WebGL2、Apple GPU」。パス・ユーザー名は写っていない)。
    `--manual` の 1 回目は「starts in Japanese with the sample profile」でアプリが応答せずに失敗し、2 回目は通った (並行するほかの作業で混んでいたためと見る)。
  - 検査: `just e2e --only studio` (13 個)・`just ui-audit --only studio` (183 場面で 0 件)・`just ci` が通った (`just ci` は main に rebase した後)。
- **やらなかったこと:** スクリーンリーダーの実機 (NVDA・VoiceOver・Orca) での読み上げは確かめていない (text content で区切りを確かめた)。
- **起票の候補:** なし。
