---
id: T0065
title: "ui-audit: エディタのツリーのドラッグの途中 (ドロップの印) を撮る場面を足す"
status: done
milestone: M10
crates: [tests, docs]
depends_on: [T1675, T0064]
assignee: claude-worker-T0065
---

# T0065: ui-audit: エディタのツリーのドラッグの途中 (ドロップの印) を撮る場面を足す

## 目的

T1670 でツリーの行のドロップの印 (`drop-into`・`drop-above`・`drop-below`・`drop-bad`) を強制色でも描くようにしたが、ドラッグの途中を写す
`just ui-audit` の場面が無く、T1670 では一時的なページで確かめただけだった。コントラスト・強制色で印が見えるかを監査で見続けるために場面を足す。

## 受け入れ条件

- [x] `tests/ui-audit/lib/apps.mjs` のエディタに、ツリーの行をポインタで押して動かし、離さずに撮る場面を足した (TreePanel.svelte の
      ポインタのドラッグ。少なくとも中へ (`drop-into`)・落とせない所 (`drop-bad`) の 2 つ。手前・奥も撮るかは場面の数との兼ね合いで決める)。
      撮った後にドラッグを取り消し (Esc や `pointercancel`)、後の場面に残さない。作り方をコメントに書き、tests/ui-audit/README.md の場面の一覧に書いた
- [x] 場面にドロップの印が写っていることを確かめた (撮った画像か、場面の中で `.row.drop-*` を待つ検査で)。強制色の組の画像も見た
- [x] `just ui-audit --only editor` を流し、足した場面の指摘を直したか、意図したもの (理由) としてメモに書いた
      (直すのがアプリのコードなら、範囲の外なので起票の候補にする)
- [x] `just ui-audit-check` と `just ci` が通る

## 参考

- T1670 のメモ (強制色のドロップの印・起票の候補)、apps/editor/src/lib/panels/TreePanel.svelte (`onRowPointerDown`・`onPointerMove`・`cancelDrag`)、
  apps/editor/src/lib/panels/tree/TreeItem.svelte (`.row.drop-*`)
- tests/ui-audit/lib/apps.mjs、tests/ui-audit/README.md

## メモ

- T0063 で起票した (元の候補: T1670 (ドラッグの途中を撮る ui-audit の場面が無い))。
- 同じツリーの行の強制色の CSS を直す T1675 と、同じ tests/ui-audit を触る T0064 の後にした。

### やったこと (claude-worker-T0065)

- `tests/ui-audit/lib/apps.mjs` に `treeDragScenes` を足し、エディタの `-tree-hidden-hover` の後で `-tree-drop-into`・`-tree-drop-above`・`-tree-drop-bad` を撮る。
  パーツの木で選んでいないメッシュの行を `page.mouse.down` で押して 10px 動かし (5px でドラッグになる)、離さずに見えている行 (押した行を除く) の
  中ほど・上半分へ順に動かして、`.row.drop-into:not(.drop-bad)`・`.row.drop-above:not(.drop-bad)`・`.row.drop-bad` が出たところで撮る。
  手前に落とせる組 (同じ描画の順のメッシュ) が無ければ次のメッシュの行 (6 つまで) から押し直し、出ない印があれば場面の失敗にする。
  撮った後は木に `pointercancel` を送って (`cancelDrag`) 印と `.tree.dragging` が消えるのを待ち、木の外へ動かしてから `mouse.up` で離す。
  落とせない所は理由の文が `.hint.error` で出るので `errors: true`。見本では、中へ = HairFront を「Body」へ、手前 = 描画の順 540 の
  BrowR/BrowL の組、落とせない所 = HairFront を HairBack (描画の順の違い) へ、になった。
- 奥 (`drop-below`) は手前と同じ線の上下違い (強制色でも `border-top`/`border-bottom` の違いだけ) なので撮らない (13 組 × 1 場面を省いた)。
- README の場面の一覧に足し、エディタの場面の数と時間を直した (568 → 604 の場面、3 並行で 6 分 40 秒。ロードアベレージ 8 ほど)。

### 確かめたこと

- `just ui-audit --only editor` (13 組・604 場面): 指摘 0、場面の失敗 0。足した 36 場面 (12 組 × 3。`en-light-half` 等の組は撮らない設定のもの以外) すべてで印が出た
  (出なければ失敗にしている)。
- 画像: `en-light-wide`・`en-light-narrow` で中へ (accent の枠と地)・手前 (上の 2px の線)・落とせない所 (danger の枠と地と理由の文)。
  強制色 `en-dark-forced-wide`・`ja-light-forced-narrow` で中へ (Highlight の破線)・手前 (Highlight の上の線)・落とせない所 (GrayText の点線) が見える。
  後の場面 (`-selected` など) に印・理由の文は残っていない。
- `just ui-audit-check`・`just ci` 通過。

### 起票の候補

- エディタのツリーの下の欄 (`TreePanel.svelte` の `.footer`。`position: sticky; bottom: 0`) が左の列の内側の余白の分だけ下端から浮き、狭い幅で
  ドラッグの理由の文が出ると、欄の下の余白 (12px ほど) に木の行 (「HandW… hidden 700」) が覗く (`en-light-narrow-tree-drop-bad`)。
  頭 (`.head`) と同じく `bottom: calc(-1 * var(--ugoe-space-3))` などで列の下端に付けるか。監査の重なり・切れの検査は、送る枠の端で切れた行として数えないので出ない。
  アプリのコードなので範囲の外。
