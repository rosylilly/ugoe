---
id: T1661
title: "エディタ: 強制色でパーツの木のフォーカスの印が前と後の比 3:1 に届かないのを直す"
status: todo
milestone: M10
crates: [apps/editor, docs]
depends_on: [T1654]
assignee:
---

# T1661: エディタ: 強制色でパーツの木のフォーカスの印が前と後の比 3:1 に届かないのを直す

## 目的

`just ui-audit --only editor` の `en-dark-forced-wide-tab` で、パーツの木 (`div[tree] "Parts"`、Tab の止まり 10) が
`faint focus ring (8.73:1 at most)` と出る (T0045 の画像の判定。T0047・T0048・T0049 でも毎回出ている)。
T0045 の調べ: 強制色では `.tree` の透明な 1px の枠が文字の色 (白) で描かれ、`.tree:focus-visible` の `outline-offset: -2px` の輪がその枠に
重なって、白が強調の色 (水色。白との比 1.2) に変わるだけになる。比の最大 8.73 は角の丸みの所の少しの画素で、3:1 以上変わった画素は要る量の 0.54 倍。
判定 (閾値) ではなく印の方が足りない (WCAG 2.4.13 の考え方)。

## 受け入れ条件

- [ ] 強制色で、パーツの木 (とデフォーマの木など同じ `.tree` の規則を使うもの) のフォーカスの輪が、強制色で描かれる枠と重ならずに出る
      (例: 強制色で `outline-offset` を外に出す・枠を描かない。送る枠で輪が切れないことも確かめる)。ライトとダークの見た目は変えないか、変えるなら理由を書く
- [ ] `just ui-audit --only editor` の `en-dark-forced-wide-tab` で `faint focus ring` が 0 件、ほかの組 (とくに `en-light-wide-tab`) で増えた指摘が無い (`--compare`)
- [ ] `focus` の判定が 0 件になったことを前と後の画像 (`<組>-focus-<番号>-before.png`・`-after.png`) で確かめ、メモに書いた
- [ ] docs/ui.md (フォーカスの印・強制色) に直し方を書いた
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0045 のメモ (CSS の判定と画像の判定の食い違い・起票の候補)、T0047・T0049 のメモ (起票の候補)、tests/ui-audit/README.md の「フォーカスの印の判定」
- apps/editor/src/lib/panels/tree/TreePanel.svelte (`.tree:focus-visible`)

## メモ

- T0052 で起票した (元の候補: T0045 (TreePanel の `.tree:focus-visible` の強制色の輪)・T0047 と T0049 (同じ場面の `faint` の理由を確かめる))。
  `faint` になる理由は T0045 のメモで分かっている (印が足りない) ので、直すタスクにまとめた。
- 作業中の T1654 (ツリーの行の札とコントラスト) と同じツリーのファイル・テストを触るので、その後にした。T1654 の範囲には木のフォーカスの印は入っていない。
