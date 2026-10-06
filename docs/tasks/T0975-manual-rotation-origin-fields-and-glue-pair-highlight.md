---
id: T0975
title: "マニュアル: 回転デフォーマの原点の数値の欄・「Origin: all cells」が残ること・グルーの組の行の強調を書き足す"
status: in-progress
milestone: M10
crates: [docs]
depends_on: []
assignee: claude-worker-T0975
---

# T0975: マニュアル: 回転デフォーマの原点の数値の欄・「Origin: all cells」が残ること・グルーの組の行の強調を書き足す

## 目的

T1649 (回転デフォーマの原点の数値の欄) と T1651 (グルーの組の行の強調) で変わった画面を、マニュアル (docs/manual) に書き足す。
どちらのタスクも docs/editor.md は直したが、マニュアルは「マニュアルの更新の候補」として残した。

## 受け入れ条件

- [ ] docs/manual/tools.md の回転のギズモの節に、インスペクタの「Origin X (this cell)」「Origin Y (this cell)」の欄を書いた
      (親の座標系の値・「Origin: all cells」を付けるとラベルが「(all cells)」になりすべてのセルに同じだけ足す・キーの間は空で押せない)。
      「Origin: all cells」がデフォーマを選び直しても残ることも書いた
- [ ] docs/manual/tutorial.md の 3 章の中心を置き直す囲みに、「Origin: all cells」を付けて欄に数を入れても置き直せることを書いた
- [ ] docs/manual/dress-up.md のグルーの節 (「Pairs」の一覧) に、行にポインタを乗せる・フォーカスするとビューポートでその組が強調されることを書いた
- [ ] 文言 (ボタン・欄の名前) がエディタの英語の文言 (`apps/editor/src/lib/i18n`) と合っている。画像は撮り直さない (撮るなら T0968 に書く)
- [ ] `just ci` が通る

## 参考

- T1649 のメモ (やらなかったこと・マニュアルの更新の候補)、T1651 のメモ (マニュアルの更新の候補)
- docs/manual/tools.md・tutorial.md・dress-up.md、docs/editor.md (インスペクタの表・グルーの画面)

## メモ

- T0052 で起票した (元の候補: T1649 (tools.md・tutorial.md)・T1651 (dress-up.md))。マニュアルの文の書き足しなので 1 つにまとめた。
- 逆向きの強調 (ビューポートの組 → パネルの行) は T1664 が入れ、そのマニュアルも T1664 で書く。
