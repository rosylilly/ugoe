---
id: T1661
title: "エディタ: 強制色でパーツの木のフォーカスの印が前と後の比 3:1 に届かないのを直す"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: [T1654]
assignee: claude-worker-T1661
---

# T1661: エディタ: 強制色でパーツの木のフォーカスの印が前と後の比 3:1 に届かないのを直す

## 目的

`just ui-audit --only editor` の `en-dark-forced-wide-tab` で、パーツの木 (`div[tree] "Parts"`、Tab の止まり 10) が
`faint focus ring (8.73:1 at most)` と出る (T0045 の画像の判定。T0047・T0048・T0049 でも毎回出ている)。
T0045 の調べ: 強制色では `.tree` の透明な 1px の枠が文字の色 (白) で描かれ、`.tree:focus-visible` の `outline-offset: -2px` の輪がその枠に
重なって、白が強調の色 (水色。白との比 1.2) に変わるだけになる。比の最大 8.73 は角の丸みの所の少しの画素で、3:1 以上変わった画素は要る量の 0.54 倍。
判定 (閾値) ではなく印の方が足りない (WCAG 2.4.13 の考え方)。

## 受け入れ条件

- [x] 強制色で、パーツの木 (とデフォーマの木など同じ `.tree` の規則を使うもの) のフォーカスの輪が、強制色で描かれる枠と重ならずに出る
      (例: 強制色で `outline-offset` を外に出す・枠を描かない。送る枠で輪が切れないことも確かめる)。ライトとダークの見た目は変えないか、変えるなら理由を書く
- [x] `just ui-audit --only editor` の `en-dark-forced-wide-tab` で `faint focus ring` が 0 件、ほかの組 (とくに `en-light-wide-tab`) で増えた指摘が無い (`--compare`)
- [x] `focus` の判定が 0 件になったことを前と後の画像 (`<組>-focus-<番号>-before.png`・`-after.png`) で確かめ、メモに書いた
- [x] docs/ui.md (フォーカスの印・強制色) に直し方を書いた
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0045 のメモ (CSS の判定と画像の判定の食い違い・起票の候補)、T0047・T0049 のメモ (起票の候補)、tests/ui-audit/README.md の「フォーカスの印の判定」
- apps/editor/src/lib/panels/tree/TreePanel.svelte (`.tree:focus-visible`)

## メモ

- T0052 で起票した (元の候補: T0045 (TreePanel の `.tree:focus-visible` の強制色の輪)・T0047 と T0049 (同じ場面の `faint` の理由を確かめる))。
  `faint` になる理由は T0045 のメモで分かっている (印が足りない) ので、直すタスクにまとめた。
- 作業中の T1654 (ツリーの行の札とコントラスト) と同じツリーのファイル・テストを触るので、その後にした。T1654 の範囲には木のフォーカスの印は入っていない。

- **結果 (T1661):** コードは変えていない。T1654 (14859c22) の `TreePanel.svelte` の強制色の規則 (`.tree:not(.drop-root)` の枠を `Canvas`) で
  足りていることを確かめ、docs/ui.md の「強制色」に直し方の決まり (透明の枠の内側に輪を重ねない) を書いた。
  - 案は「枠を地の色にする」(T1654 が採った) と「輪を外に出す」の 2 つ。輪を外に出すと左の列の横の余白と送る枠に依るので、内側のまま枠を消す方を残した。
- **前と後の比べ方:** 作業ツリーで T1654 の強制色の規則だけを一時的に外して `--out target/ui-audit-before` を撮り、戻して `--compare` した
  (`--variants en-dark-forced-wide,en-light-wide`、90 場面)。結果は `editor: +0 -1 ~0` (消えたのは `div[tree] "Parts"` の
  `faint focus ring (8.73:1 at most)`、stop 10)。後はすべての検査で 0 件。
- **画像:** 木の止まりは印ありになると画像が書かれないので、`walk.mjs` に一時的に「役割が tree の止まりも書く」条件を足して撮った (コミットしていない)。
  - `en-dark-forced-wide-focus-10`: 前の版は外したときも白い 1px の枠があり、フォーカスで水色に変わるだけ。後の版は外したときに枠が無く
    (地の黒)、フォーカスで水色の輪が四辺に出る。
  - `en-light-wide-focus-10`: 前と後の `-before.png`・`-after.png` がバイト単位で同じ (ライトとダークの見た目は変わらない)。
- **同じ規則を使うほかの木:** デフォーマの木はパーツの木と同じ要素 (`TreePanel.svelte` の `div.tree`。タブで中身と `aria-label` だけ変わる)。
  一時的な場面 (コミットしていない) でデフォーマの木にフォーカスを置き、`en-dark-forced-wide` で水色の輪が四辺に出ることを画像で確かめた。
  ほかに `.tree` の規則を使う木は無い (`class="tree"` は TreePanel の 1 か所)。
- **送る枠:** `ja-light-forced-narrow` (1024×720) で左の列 (`aside`) を縦に送り、木にフォーカスを置いて撮った。輪は内側 (`-2px`) なので
  左右の辺は切れず、送った分だけ上の辺が見出し (sticky) の下に隠れ、送り切ると下の辺が出る。`en-dark-forced-wide` では木が列に収まり送られない。
- `just e2e --only editor` は 16 件すべて通った。`just ci` は通った。
- **確かめていないこと:** 実物の Windows の高コントラスト (WebView2) の見た目 (U04 の手順の範囲)。
- **起票の候補:** 無し。
