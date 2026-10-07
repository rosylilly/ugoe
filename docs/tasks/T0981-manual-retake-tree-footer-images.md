---
id: T0981
title: "マニュアル: T1676 の後に、ツリーの下端が変わった 5 枚の画像を撮り直す"
status: done
milestone: M10
crates: [docs, tests]
depends_on: []
assignee: claude-worker-T0981
---

# T0981: マニュアル: T1676 の後に、ツリーの下端が変わった 5 枚の画像を撮り直す

## 目的

T1676 でエディタのツリーの下の欄 (`TreePanel.svelte` の `.footer`) を左の列の下端に付けたので、木が列からあふれる画像では下端の
10px ほどの帯が変わった (覗いていた行の端が地の色になる): `ai-normal-maps.png`・`lighting-point.png`・`verify.png`・`lighting.png`・
`module-base-readonly.png`。説明の文とは食い違わないが、画面と画像をそろえるために撮り直す。

## 受け入れ条件

- [x] `just e2e --update-images --only editor` で撮り直し、上の 5 枚 (と、T0980 の後の画面の変更で変わった画像があればそれも) だけを入れ、
      アニメーション (呼吸・物理)・経過の秒・ポートの番号などの違いだけの画像は前のままにした。どの画像を入れたかと変わった理由をメモに書いた
- [x] 入れた画像をすべて開き、撮った機械の固有のもの (/home・worktree・mbx のハッシュ・乱数の一時フォルダ) が写っていないことを確かめた
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T1676 のメモ (マニュアルの画像)、T0980 のメモ (撮り直しの分け方)、tests/e2e/README.md#マニュアルの画像

## メモ

- T0066 で起票した (元の候補: T1676 (ツリーの下の欄の付け方を直したので、5 枚の下端 10px が変わる))。
- 急がない。ほかのタスクで画像を撮り直すことになったら、そのタスクで一緒に入れて、このタスクを閉じてよい。

### やったこと (claude-worker-T0981)

- `just e2e --update-images --only editor` (main 743b771e の上) で撮り直すと 28 枚が変わった。前の画像との差 (しきい値 3%) の場所を測り、1 枚ずつ見て分けた。
- **入れた 7 枚:**
  - ツリーの下端の 10px の帯で、覗いていた行の端が地の色になった (T1676 の下の欄の付け方): `ai-normal-maps`・`lighting-point`・`lighting`・
    `verify`・`module-base-readonly`。ほかの差は 3〜6px の角のにじみ (ボタンの角・取っ手) だけ。
  - 木を一番下まで送った画像で、木の行が 2px 上に寄った (T1676 で下の欄に `padding-top` が付き、送れる長さが延びた): `module-workspace`・
    `module-export-atlas` (左の木の部分)。もう一度 `just e2e --manual --only editor` で撮っても同じ位置になる (`module-workspace` は差 0 画素、
    `module-export-atlas` は 105 画素のにじみだけ) ので、画面の変更による差と見た。T0980 の後のエディタの画面の変更は T1676 だけ (ほかは ui-audit の検査の変更)。
- **戻した 21 枚:** 呼吸・物理・ポーズのアニメーションとパラメータの値の差 (`preview`・`preview-tracking`・`preview-module-pose`・`dress-up-preview`・
  `module-compare-highlight`・`module-compare-pose`・`physics-preview`・`depth-rotation` の描画)、重ねて描くスクロールバーが出ているかどうかの差
  (`screen`・`timeline`・`tools-mesh-edit`・`depth-rotation` の右端)、MCP の待ち受けのポートの番号・経過の秒 (`mcp-connect`・`mcp-claude-desktop`・
  `mcp-approval`)、数画素の角のにじみだけ・差 0 (`ai-proposal`・`keyform-grid`・`module-glue`・`tools-rotation`・`physics-rig`・`timeline-settings`・
  `tools-warp-gizmo`)。
- 入れた 7 枚をすべて開き、/home・worktree・mbx のハッシュ・乱数の一時フォルダが写っていないことを見た (パスの出る画面は無い)。
  PNG の付帯情報と中の文字列にも無い (`identify -verbose`・`strings`)。
- 説明の文 (docs/manual) と食い違う所は無い (文は直していない)。

### 確かめたこと

- `just e2e --update-images --only editor`: 通過。`just e2e --only editor`: 16 件通過。
- `just ci`: 通過 (main 743b771e の上。前面で実行。終了コード 0)。

### 起票の候補

- なし。
