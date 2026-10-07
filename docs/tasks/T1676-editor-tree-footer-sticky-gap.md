---
id: T1676
title: "エディタ: ツリーの下の欄 (sticky) が左の列の余白の分だけ下端から浮き、その下に木の行が覗く"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1676
---

# T1676: エディタ: ツリーの下の欄 (sticky) が左の列の余白の分だけ下端から浮き、その下に木の行が覗く

## 目的

ツリーの下の欄 (`TreePanel.svelte` の `.footer`、`position: sticky; bottom: 0`) が、左の列の内側の余白の分だけ下端から浮いている。
狭い幅で理由の文 (`.hint.error`) が出ると、その下の 12px ほどに木の行が覗く (ui-audit の `en-light-narrow-tree-drop-bad` の画像。T0065 の候補)。
監査の検査では出ない。

## 受け入れ条件

- [x] 下の欄を左の列の下端に付け、下に木の行が覗かないようにした (`.head` と同じく `bottom` を負の余白にするなど)。見出し (`.head`) の扱いとそろえた
- [x] `just ui-audit --only editor --variants <狭い幅を含む組>` で、`-tree-drop-bad` などの画像で直ったことを見た。指摘は増えない (`--compare`)
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0065 のメモ、apps/editor/src/lib/panels/tree/TreePanel.svelte

## メモ

- T0065 のワーカーの報告から起票した (2026-10-07)。

### やったこと (2026-10-07)

- `TreePanel.svelte` の `.footer` を頭 (`.head`) とそろえた: `bottom` と `margin-bottom` を左の列の余白 (`--ugoe-space-3`) の分だけ負にし、
  その分を `padding-bottom` にして列の下端に付けた。上には頭の下の余白と対になる `padding-top: var(--ugoe-space-1)` を付け、
  切れた行と理由の文が接しないようにした。頭と同じく `z-index: 1`。
- 下の欄が列の下端の帯 (10px ほど。下の欄が空のときも) を覆うようになったので、矢印のキーで送った行がその下に隠れないよう、
  行 (`.tree > .row`) に `scroll-margin` を頭と下の欄の高さ (`bind:offsetHeight` で測り、CSS の変数で渡す) で付けた。
  一時的な探り (ui-audit の場面に入れてコミットしていない) で確かめた: `en-light-narrow` で ArrowDown を 40 回押すと、付けない場合は行が 696–720 で
  下の欄 (710–720) に 10px 隠れ、付けた場合は 686–710 で接する。ArrowUp で戻すと、付けない場合は行の上端 96 が頭の下端 99 に 3px 隠れ
  (前からあった)、付けた場合は 99 で接する。
- docs/ui.md の「送り」に、余白のある送る枠で貼り付けるものの付け方と `scroll-margin` を書いた。
- 確かめ: `just ui-audit --only editor --variants narrow,wide` を直す前 (`--out target/ui-audit-before`) と後で比べ、`--compare` で `+0 -0 ~0` (指摘は 0 のまま)。
  `en-light-narrow-tree-drop-bad` で理由の文の下に覗いていた木の行 (「HandW… hidden 700」) が消えたことを画像で見た。`-wide-tree-drop-bad` も崩れていない。
  `just e2e --only editor` 16 件通過。`just ci` 通過。

### マニュアルの画像

- 直す前と後で `just e2e --only editor --manual` を撮り比べた。木が左の列からあふれる画像は、列の下端の 10px ほどで、覗いていた行の端が地の色になる
  (撮り直しの候補): `ai-normal-maps.png`・`lighting-point.png`・`verify.png`・`lighting.png`・`module-base-readonly.png`。
  差は小さく (下端の 10px の帯だけ)、撮り直さなくても説明とは食い違わない。ほかに差の出た画像 (`preview*.png`・`dress-up-preview.png`・
  `module-compare-*.png`・`module-workspace.png`・`mcp-*.png` など) は、ツリーの下端と関係の無い所 (描画・動き) の差で、このタスクの変更によるものではない。
  このタスクでは撮り直していない。
