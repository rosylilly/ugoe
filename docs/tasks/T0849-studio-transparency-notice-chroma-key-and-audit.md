---
id: T0849
title: "スタジオ: 透過が効かない注意に「背景をクロマキーにする」を付け、ui-audit に透過が効かない場面を足す"
status: done
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

- [x] 透過が効かない注意 (`Notice`) に「背景をクロマキーにする」のボタンがあり、押すと配信しているシーンの背景がクロマキーになる
      (シーンの背景の変更として保存され、取り消し方は既存の背景の欄と同じ)。押した後は注意が消える。部品のテストがある
- [x] `tests/ui-audit` の偽のホストに透過が効かない様子の場面 (`-opaque` など) があり、`just ui-audit --only studio` の指摘が 0 件。tests/ui-audit/README.md の場面の表を直した
      (新しい `-opaque` の場面は 12 組とも 0 件。前からある `ja-dark-narrow-approved` のコントラスト 2 種類は T0952 の候補で残る。メモ)
- [x] docs/studio.md#出力ウィンドウの透過 と docs/manual/studio.md (困ったとき) を直し、画面が変わったところを T0919 のメモに書き足した
- [x] `just ci` が通る

## 参考

- T0846 のメモ (調べた結果・やったこと・起票の候補)
- apps/studio/src/OutputPanel.svelte、apps/studio/src/lib/status.ts、tests/ui-audit/lib/studio-host.mjs

## メモ

- T0041 で起票した (元の候補: T0846 (クロマキーのボタン)・T0846 (ui-audit の場面))。T0952 (作業中) も tests/ui-audit を触るので、その後に着手する。
- **やったこと:**
  - `OutputPanel.svelte`: 透過が効かない注意 (`Notice`) の `actions` に「背景をクロマキーにする」(`output.opaqueChroma`) のボタンと、新しい `onchroma`。
    注意の文も「下のボタンで…」に直した (英語・日本語)。
  - `Control.svelte`: `chromaLiveBackground` で、配信しているシーン (編集中の写しがあればそれ) の背景を `withBackgroundKind(…, "chroma")` (既定の緑) にして
    `updateScene` に渡す。「シーンの編集」の背景の欄と同じ送り方なので、保存され、戻すときは「シーンの編集」の「背景」で「透過」を選ぶ (取り消しの仕組みは別に作っていない)。
    背景が透過でなくなるので注意は消える。
  - 部品のテスト (`OutputPanel.svelte.test.ts`): ボタンが `onchroma` を呼び、背景がクロマキーになると注意が消える。
  - tests/ui-audit: 偽のホストに `setOutput` (出力の様子を変えて `output` の通知を送る) を足し、`-output` の後に `-opaque` の場面 (透過を `unsupported`、
    撮った後に `supported` へ戻す)。README の場面の表と偽のホストの説明を直した。
  - tests/e2e (範囲の外ではない。`tests`): studio の透過の手順で、`unsupported` ならボタンを押して背景がクロマキーになり注意が消えること、
    「シーンの編集」で「透過」に戻すと注意がまた出ることを確かめる (後の手順は透過の「雑談」のまま)。README の筋書きを直した。
  - 文書: docs/studio.md#出力ウィンドウの透過、docs/manual/studio.md (はじめての配信の準備・困ったとき)、docs/manual-check.md の S28 (手順と期待)、
    apps/studio/README.md、T0919 のメモ。
- **確かめたこと:** `just ui-audit --only studio --out target/ui-audit-before` (変更の前の場面) → 変更の後に `--compare`: `+0 -0 ~0`、134 場面
  (`-opaque` が 12)、`-opaque` の指摘は 0 件。撮った画像 (ライトの日本語の広い幅・ダークの英語の狭い幅) で、注意の中にボタンが収まることを見た。
  `just e2e --only studio` が rebase の前と後で 13 件とも通った (Xvfb で本物のスタジオが `unsupported` を返し、ボタンで背景が替わった)。
- **残る ui-audit の指摘 (この変更と無関係):** `ja-dark-narrow-approved` の `span.scenes__model`・`span.ugoe-kbd__plus` のコントラスト 4.29
  (選んだ行の hover の重ね。T0952 のメモの起票の候補)。変更の前の基準線にもあり、hover のトークンの決め方 (T0916) に関わるので直していない。
- **確かめていないこと:** 実際の X11 のデスクトップでコンポジタを止めてボタンを押す流れ (S28 に手順を足した)。
- **`just ci`:** rebase の後 (main 16971f65 の上) で通った。
- **起票の候補:** なし (上の hover のコントラストは T0952 の候補のまま)。
