---
id: T1654
title: "エディタ: ツリーで base と hidden の札を両方持つ行の名前が 1 文字に縮むのと、選んだ非表示・本体の行のホバーのコントラストを直す"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: [T0048]
assignee: claude-worker-T1654
---

# T1654: エディタ: ツリーで base と hidden の札を両方持つ行の名前が 1 文字に縮むのと、選んだ非表示・本体の行のホバーのコントラストを直す

## 目的

ツリーの行 (`apps/editor/src/lib/panels/tree/TreeItem.svelte`) の 2 つの不具合を直す。

- モジュールのプロジェクトで、`base` と `hidden` の 2 つの札 (`.ed-tag`) が付く行の名前が 1 文字 (「E…」) まで縮む (T0919。本物の WebKitGTK で見つけた)。
- 非表示 (`.row.hidden`)・本体 (`.row.base`) の要素を選んだ行にポインタを乗せると、名前 (text-muted) が accent-soft と hover の重ねの上で
  ダーク 4.29 になる (T0851 の計算。4.5:1 に届かない)。T0851 のスタジオのシーンの一覧と同じ直し方 (選んだ行では本文の色へ寄せる) が使える。

## 受け入れ条件

- [x] 札を 2 つ持つ行でも、既定の幅の右の列で名前が読める長さを保つ (札を縮める・1 つの印にまとめる・名前を優先して札を省く、など。
      選んだ形と理由をメモに書いた)。札の意味は読み上げ (`title` か見せずに読ませる文字) で残る
- [x] 選んだ非表示・本体の行の hover・押下でも名前が 4.5:1 以上 (ライト・ダーク)。トークン (`--ugoe-overlay-hover`) は変えない
- [x] T0048 で足した場面を含む `just ui-audit --only editor` で、ツリーのコントラストの指摘が 0 件、`--compare` で増えた指摘が無い
- [x] 部品のテスト (TreePanel.test.ts など) で、札を 2 つ持つ行の名前と札の読み上げを確かめた
- [x] docs/ui.md (エディタのパネル) に直し方を書いた。マニュアルの更新の候補をメモに書いた
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」、T0851 のメモ (直し方・起票の候補)
- apps/editor/src/lib/panels/tree/TreeItem.svelte、docs/ui.md

## メモ

- T0046 で起票した (元の候補: T0919 (札 2 つの行の名前が 1 文字に縮む)・T0851 (選んだ非表示・本体の行の hover のコントラスト))。
  同じ TreeItem.svelte の行の見た目なので 1 つにまとめた。
- 監査に場面が無いので、場面を足す T0048 の後にした。
- T0048 で場面 `-tree-hidden-hover`・`-tree-base-hover` を足した。非表示の要素を選んだ行のホバーはダークの 4 組で 4.29 (名前「Wave」と札「hidden」)。本体の要素を選んだ行のホバーは**ライト**の 6 組で 4.44 (名前「Head」と札「base」。地が sunken のため)、ダークは 0 件。両方を直す対象にする (T0048 のメモ)。

### やったこと (claude-worker-T1654)

- **札 2 つの行 (選んだ形):** 本体の札とほかの札 (非表示・マスク) が並ぶ行では、本体の札を出さず、ほかの札の枠を破線にする
  (本体の要素の選択と同じ破線。`TreeItem.svelte` の `tags`・`baseMerged`・`.ed-tag.base-merged`)。「本体」の文字は札の中に
  `.ugoe-visually-hidden` で置き、読み上げは「base hidden」のまま。`title` は 2 つの説明を改行でつなぐ。見本のモジュールのプロジェクトの
  既定の幅で、名前が「Ey…」→「EyeRClo…」、「S…」→「Sleev…」、「Ha…」→「HandWave」になった (`en-light-wide-tree-base-hover` の画像)。
  理由: モジュールのプロジェクトでは本体の行すべてに「本体」の札が付くので、例外の印 (非表示) を残す方が情報が多い。札を縮める (「hid…」)・
  1 つの札に 2 語を並べる (15px ほどしか減らない) 案は不十分、本体の札を黙って省くと本体とモジュールの非表示の行が見分けられない。
- **選んだ行の hover:** 行の変数 `--tree-muted` を、選んだ行では text-muted に本文の色を 30% 混ぜた色にし (T0851 と同じ)、名前・札・描く順に使う。
  ダークの accent-soft + hover で約 5.4、ライトの sunken + hover で約 6.2 (計算)。トークンは変えていない。
- **T0045 の強制色の `faint focus ring` (直した):** 強制色では `.tree` の透明な枠が白で描かれ、内側に重なる輪との比が 1.2 だった。
  `@media (forced-colors: active)` で `.tree:not(.drop-root)` の枠を `Canvas` にした (`TreePanel.svelte`)。
- 部品のテスト: `TreeItem.test.ts` に、本体 + 非表示の行で見える札が 1 つ・`base-merged`・読み上げの文字・`title`・名前、札 1 つの行は前のまま、を足した。
- docs/ui.md に「エディタのツリーの札と選んだ行 (T1654)」、docs/editor.md の状態の表 (本体の要素) を直した。

### 確かめたこと

- `just ui-audit --only editor --variants en-dark-wide,ja-dark-narrow,en-light-wide,ja-light-narrow,forced` (6 組・266 場面) を、
  前 (この worktree の着手時 1f175adf) と後で `--compare`: `editor: +0 -3 ~0` (`span.ed-tag`・`span.name` の 4.44/4.29 と
  `div[tree] "Parts"` の `faint focus ring` が消え、すべての検査で 0 件)。前面の 600 秒に収めるため 13 組すべては流していない
  (ほかの組: ダークの広い幅の日本語・狭い幅の英語・200% の拡大など)。
- `just e2e --only editor`: 16 件すべて通過。`just ci` 通過 (rebase の後)。
- 本物の WebKitGTK では見ていない (札 2 つの行の縮み方はヘッドレスの Chrome の画像で確かめた)。

### マニュアルの更新の候補

- docs/manual/dress-up.md (147 行目「本体の要素に `base` の印が付いて薄く表示されます」) に、「非表示などほかの印もある行では `base` の印は出さず、
  その印の枠が破線になる」と書き足す。モジュールのプロジェクトのツリーを写した画像 (module-base-readonly.png など) は撮り直すと札が変わって写る。

### 起票の候補

- 札 (`.ed-tag`) が 2 つ並ぶほかの組み合わせ (本体でない、マスク専用かつ非表示の行) は、今の見本には無いが同じく名前が縮む。起きるなら同じ形にまとめる。
- 強制色で、ツリーの行の透明な 1px の枠 (`.row`) もすべて白で描かれて行ごとに箱になる (`en-dark-forced-wide` の画像)。枠を `Canvas` にするか、
  選択・キーボードの位置だけ枠を出すかを決める (強制色での選んだ行の見分け方に関わるので、この範囲では変えなかった)。
