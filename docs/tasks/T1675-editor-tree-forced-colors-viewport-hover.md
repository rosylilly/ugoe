---
id: T1675
title: "エディタ: 強制色で、ツリーのビューポートからのホバー (hovered・hovered-within) が見えないのを直す"
status: done
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

- [x] 強制色で `hovered` と `hovered-within` を見分けられる形 (枠の左辺の線・`Highlight` の枠など、システムの色だけで) を決め、
      `@media (forced-colors: active)` で描いた。選択・キーボードの位置・本体の行・ドロップの印 (T1670) と重なったときにも見分けられる。
      行にポインタがあるホバー (`:hover`) も描くかを決め、理由をメモと docs/ui.md の「エディタのツリーの行の札と強制色」に書いた
- [x] 強制色でないときの見た目は変わらない
- [x] `just ui-audit --only editor --variants forced` (と強制色でない組を 1 つ) を直す前と `--compare` で比べ、`-tree-hover`・
      `-tree-hover-collapsed` の画像で印が見えることを確かめた
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T1670 のメモ (強制色の行・起票の候補)、apps/editor/src/lib/panels/tree/TreeItem.svelte (`.row.hovered`・`.row.hovered-within`・強制色の節)
- docs/ui.md (強制色の節)、docs/editor.md#選択とホバーの見せ方
- tests/ui-audit/lib/apps.mjs の `tree-hover`・`tree-hover-collapsed` の場面

## メモ

- T0063 で起票した (元の候補: T1670 (強制色でツリーのホバーと畳んだ行の中のメッシュの印が出ない))。
- 本物の Windows の高コントラストと WebView2 では見られないなら、Chrome の強制色のエミュレーションで確かめた範囲をメモに書く。

### やったこと (claude-worker-T1675)

- **強制色のホバー:** `TreeItem.svelte` の `@media (forced-colors: active)` に、ホバー (`:hover`・`hovered`・`hovered-within`) は名前の 2px の下線、
  畳んだ行の中のメッシュ (`hovered-within`) は下線に加えて左端の 3px の縦線 (`::before` の左の枠で。`Highlight`、選んだ行では `HighlightText`) を足した。
  枠・地を使わない形なので、選択 (地)・キーボードの位置 (枠)・本体の選択 (破線の枠)・ドロップ (破線・上下の線・点線) と重なっても見分けられる。
- **`:hover` も描く:** 行に乗ると `Selection.hover` が書かれて `hovered` も付くので、`:hover` だけ外すと形が変わらない (分ける意味が無い)。
  ビューポートから乗ったときと同じ形の方が対応が分かるので、強制色でないときの規則と同じく `:hover` も含めた。
- **文書:** docs/ui.md の「エディタのツリーの行の札と強制色 (T1670)」の表に 2 行と、理由の段落を足した。docs/editor.md#選択とホバーの見せ方 の強制色の文も直した。
- テストの追加は無し (CSS の強制色の規則だけで、jsdom では強制色を評価できない。確かめは下の監査と画像)。

### 確かめたこと

- `just ui-audit --only editor --variants en-dark-wide,ja-light-narrow,forced` (4 組・189 場面) を直す前 (`target/ui-audit-before`) と後で
  `--compare`: `editor: +0 -0 ~0`。強制色でない組の `-tree-hover`・`-tree-hover-collapsed` の画像は前と後でバイト単位で同じ。
- 画像: `en-dark-forced-wide-tree-hover-collapsed` で畳んだ「Head」に左端の縦線と下線、`ja-light-forced-narrow-tree-hover` で「Head」に下線が出る。
- 監査に無い重なり (選択・選択 + 位置・本体の選択・ドロップの中へ・手前・奥・落とせない、それぞれとホバー / 畳んだ行の中のホバー) は、
  ビルドした CSS で行を並べた一時的なページをヘッドレスの Chrome の強制色 (ライト・ダーク) で撮って見分けられることを確かめた。
  ダークの選んだ行の縦線は `HighlightText` (黒) なので地の切れ目に見えるが、位置と太さで分かる。
- `just e2e --only editor` 通過、`just ci` 通過 (rebase の後)。
- 本物の Windows の高コントラストと WebView2 では見ていない (Chrome の強制色のエミュレーションだけ)。

### 起票の候補

- 無し。
