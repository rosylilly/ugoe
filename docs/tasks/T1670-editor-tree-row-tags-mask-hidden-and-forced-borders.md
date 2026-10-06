---
id: T1670
title: "エディタ: ツリーの「マスク専用かつ非表示」の行で札が 2 つ並んで名前が縮むのと、強制色で行の透明な枠が箱に見えるのを直す"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1670
---

# T1670: エディタ: ツリーの「マスク専用かつ非表示」の行で札が 2 つ並んで名前が縮むのと、強制色で行の透明な枠が箱に見えるのを直す

## 目的

T1654 で、本体の行にほかの印 (非表示など) もあるときは `base` の札を出さず、その札の枠を破線にして 1 つにまとめた (行の名前を縮めないため)。
ただし本体でない行でも、マスク専用かつ非表示なら `TreeItem.svelte` の `tags` に `mask` と `hidden` の札が 2 つ並び、同じく名前が縮む
(今の見本にその行が無いので、ui-audit にも出ていない)。

また強制色では、ツリーの行 (`.row`) の透明な 1px の枠がすべて文字の色 (白) で描かれ、行ごとに箱に見える (`en-dark-forced-wide` の画像)。
T1654 は木 (`.tree`) の枠だけを `Canvas` にした。行の枠は、強制色での選んだ行・キーボードの位置の見分け方に関わるので、合わせて決める。

## 受け入れ条件

- [x] マスク専用かつ非表示の行を、部品のテスト (TreeItem か TreePanel) と、見本か偽のホストの行で再現した。札の並べ方 (1 つにまとめる・
      片方を名前の飾りにするなど) を決めて直し、読み上げで両方の印が伝わる (T1654 の本体の札と同じ考え方)
- [x] 強制色での行の枠を決めて直した (例: 枠を `Canvas` にし、選んだ行とキーボードの位置だけ `Highlight` の枠を出す)。
      強制色で選んだ行・キーボードの位置・ドロップの位置が、色だけでなく枠や地で見分けられることを確かめた
- [x] 決めたことと理由を docs/ui.md (エディタのツリーの札と選んだ行・強制色) に書いた
- [x] `just ui-audit --only editor` (強制色の組を含む) で増えた指摘が無い。`just e2e --only editor` と `just ci` が通る

## 参考

- T1654 のメモ (やったこと・起票の候補)、T1661 のメモ (木のフォーカスの印と強制色)、T1667 (本体の札の見えない文)
- apps/editor/src/lib/panels/tree/TreeItem.svelte (`tags`・`baseMerged`)、apps/editor/src/lib/panels/TreePanel.svelte (`@media (forced-colors: active)`)
- docs/ui.md#エディタのツリーの札と選んだ行-t1654

## メモ

- T0058 で起票した (元の候補: T1654 (マスク専用かつ非表示の行の札が 2 つ並ぶ)・T1654 (強制色で行の透明な枠が箱に見える))。
  どちらも同じ TreeItem.svelte の行の見た目なので 1 つにまとめた。

### やったこと (claude-worker-T1670)

- **札の並べ方:** `TreeItem.svelte` の札を、本体・マスク・非表示の順に集めて、見せるのは最後の 1 つ (非表示 > マスク > 本体)、ほかは文字を見せずに
  その札の中に置く (`.ugoe-visually-hidden`) 形にまとめた (T1654 の本体の札のまとめ方を一般にした)。読み上げは「mask hidden」「base mask hidden」、
  `title` はすべての説明を改行でつなぐ。まとめたマスクは行の印 (マスクの印) と斜体の名前で見える。見せない文字の位置の基準 (`position: relative`) は
  すべての札に移した (T1667 の基準を、本体でない札にも)。
- **強制色の行:** `@media (forced-colors: active)` で、行の枠を `Canvas`、選んだ行は `Highlight` の地と `HighlightText` の文字・札・印
  (`forced-color-adjust: none`。強制のままだと文字の後ろの backplate で `Highlight` の上に黒い箱ができた)、本体の選んだ行は `HighlightText` の破線、
  キーボードの位置は `CanvasText` の枠 (選んだ行なら内側に `HighlightText` の線も)、ドロップの中へは `Highlight` の破線、手前・奥へは上下の 2px の線
  (内側の影は強制色で消えるため)、落とせない所は `GrayText` の点線にした。
- **テスト:** `TreeItem.test.ts` に、マスク専用かつ非表示の行 (札 1 つ・読み上げ「mask hidden」・`title`)、本体 + マスク + 非表示、本体 + マスクを足した。
- **偽のホスト (範囲外の小さな変更):** `tests/ui-audit/lib/editor-fixtures.mjs` の `maskOnlyHiddenModel` で、見本に `HandWaveMask` (非表示のパーツ
  「Wave」の手を写したマスク専用のメッシュ) を足した (`editor-host.mjs`・`hosts.test.mjs`・README)。`-tree-hidden-hover`・`-tree-base-hover` に写る。
- **文書:** docs/ui.md に「エディタのツリーの行の札と強制色 (T1670)」と、強制色の節に行の枠と `forced-color-adjust: none` の決まりを足した。
  docs/editor.md の状態の表、docs/manual/screen.md の札の説明も直した。

### 確かめたこと

- `just ui-audit --only editor --variants en-dark-wide,ja-light-narrow,forced` (4 組・185 場面) を、前 (main 2f1764e0 の一時的な worktree) と後で
  `--compare`: `editor: +0 -0 ~0`。途中の版では、選んだ本体の行の地が `Canvas` のまま (`.row.base.selected` の方が詳しい) で
  `span.name "Head"`・`span.ed-tag "base"` の 1.00 が出て、直した。前面の 600 秒に収めるため 13 組すべては流していない。
- 画像: `en-dark-forced-wide-tree-hidden-hover` で行の箱が消え、選んだ「Wave」が `Highlight` の地に黒い文字、「HandWaveMask」の札が「hidden」1 つ。
  `-tree-base-hover` で選んだ「Head」が破線、「HandWaveMask」が破線の「hidden」1 つ。
- キーボードの位置・ドロップの位置は監査の場面に無いので、ビルドした CSS で行の各状態 (選択・キーボードの位置・選択 + 位置・本体の選択・
  ドロップの中へ・手前・奥・落とせない) を並べた一時的なページを、ヘッドレスの Chrome の強制色 (ライト・ダーク) と強制色なしで撮って見分けられることを確かめた
  (強制色なしの見た目は前と変わらない)。
- `just e2e --only editor`: 16 件すべて通過。`just ci` 通過 (rebase の後。rebase の後は `just wasm` を流し直した: 前の生成物が T0492 の前のもので
  `wasm-types` が食い違ったため)。
- 本物の Windows の高コントラストと WebView2 では見ていない (Chrome の強制色のエミュレーションだけ)。

### 起票の候補

- 強制色で、ツリーのホバー (地の重ね) と畳んだ行の中のメッシュに乗っている印 (左端の内側の影) が出ない。ポインタの位置で分かるので足していないが、
  ビューポートからのホバー (`hovered`・`hovered-within`) は行にポインタが無いので強制色では分からない。枠の左辺などで描くかを決める。
- ドラッグの途中 (ドロップの位置) を撮る ui-audit の場面が無い。強制色のドロップの印を監査で見るなら場面を足す。

