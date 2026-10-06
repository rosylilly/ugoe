---
id: T1651
title: "エディタ: グルーのパネルの組の行にフォーカス・ホバーしたとき、ビューポートでその組を強調する"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: [T1650]
assignee: claude-worker-T1651
---

# T1651: エディタ: グルーのパネルの組の行にフォーカス・ホバーしたとき、ビューポートでその組を強調する

## 目的

T1648 でグルーのパネルに組の一覧 (「Pairs」。`A 2 ↔ B 1`) を置いたが、番号だけではビューポートのどの頂点の組かが分かりにくい。
組の行にフォーカスか、ポインタを乗せたとき、ビューポートのグルーの重ねでその組 (2 つの頂点とその間の線) を強調し、番号と絵を結び付ける。

## 受け入れ条件

- [x] 組の行 (と行の「×」) にフォーカスかポインタがあるとき、ビューポートのグルーの重ねでその組だけが強調される (色と太さ。色だけに頼らない)。
      離れると戻る。ほかの組は今のまま
- [x] 強調は表示だけで、Command を送らない (Undo の履歴に残らない)
- [x] 部品のテスト: 行にフォーカスすると強調する組が決まり、外すと消える。ビューポートの重ねが強調する組を受け取って描き分ける
- [x] docs/editor.md のグルーの画面 (「組の一覧」) に書いた。マニュアルの更新の候補をメモに書いた
- [x] `just ui-audit --only editor` (`-glue-edit`) で増えた指摘が無く、`just e2e --only editor` と `just ci` が通る

## 参考

- T1648 のメモ (起票の候補 (ほか))
- apps/editor/src/lib/panels/glues/、ビューポートのグルーの重ね (apps/editor/src/lib/tools・viewport)、docs/editor.md

## メモ

- T0046 で起票した (元の候補: T1648 (組の行にフォーカスしたときビューポートで強調))。
- 同じ GluesPanel を触るので T1650 の後にした。
- やったこと (2026-10-06):
  - `GlueView` (glueView.svelte.ts) に `hoveredPair`・`focusedPair` (組を A・B の頂点の番号で持つ) と `highlightedPair` (ポインタの組を
    先に、無ければフォーカスの組) を足した。表示だけの状態で Command は送らない。
  - GluesPanel の組の行 (`li`) の `pointerenter`/`pointerleave` と `focusin`/`focusout` (中の「×」のフォーカスが上がる) で決める。
    強調している行は枠を強調の色と outline (強制色でも残る) にする。グルーを替えたとき・パネルを閉じたとき・組が無くなったとき
    (「×」で外した行はフォーカスを外す事象を出さずに消える) は強調を消す。
  - GlueOverlay は強調する組を、ほかの組の上に `g.highlight` で描く: 暗い縁取り (`--viewport-picked-edge`、太さ 6) の上に
    `--viewport-picked` の太さ 3 の線、両端は半径 5 の点 (ほかの組は太さ 1.5・半径 2)。色と太さ・大きさで示す。ほかの組はそのまま。
  - 部品のテスト: `GluesPanel.test.ts` (「×」のフォーカスと行へのポインタで強調が決まり、外すと消える。Command は送らない。
    組が無くなった・グルーを替えたら消える)、`GlueOverlay.test.ts` (指した組だけを描き分ける・ポインタを優先・無い組は描かない)。
  - docs/editor.md のグルーの画面に「組の強調」、docs/ui.md のビューポートの色の表の `--viewport-picked` の用途に追記。
  - 確かめたこと: `just e2e --only editor` (15 件通過)、`just ui-audit --only editor --variants en-light-wide,ja-light-zoom200,ja-light-forced-narrow`
    (121 場面、すべての検査が 0)、`just ci`。
- やらなかったこと: ui-audit に行を強調した場面は足していない (今の `-glue-edit` は行にフォーカスを置かない)。実機 (Tauri の窓) では
  動かしていない (部品のテストと e2e・ui-audit の偽のホストまで)。
- マニュアルの更新の候補 (起票の候補): docs/manual/dress-up.md のグルーの節 (T1648 の候補の「Pairs」の一覧の説明) に、行に乗せる・
  フォーカスするとビューポートでその組が強調されること (番号と絵の結び付け) を足す。
- 起票の候補 (ほか): ビューポートで組の線・頂点にポインタを乗せたとき、逆にパネルの行を強調し一覧をその行まで送る (双方向の結び付け)。
