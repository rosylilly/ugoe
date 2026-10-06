---
id: T0975
title: "マニュアル: 回転デフォーマの原点の数値の欄・「Origin: all cells」が残ること・グルーの組の行の強調を書き足す"
status: done
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

- [x] docs/manual/tools.md の回転のギズモの節に、インスペクタの「Origin X (this cell)」「Origin Y (this cell)」の欄を書いた
      (親の座標系の値・「Origin: all cells」を付けるとラベルが「(all cells)」になりすべてのセルに同じだけ足す・キーの間は空で押せない)。
      「Origin: all cells」がデフォーマを選び直しても残ることも書いた
- [x] docs/manual/tutorial.md の 3 章の中心を置き直す囲みに、「Origin: all cells」を付けて欄に数を入れても置き直せることを書いた
- [x] docs/manual/dress-up.md のグルーの節 (「Pairs」の一覧) に、行にポインタを乗せる・フォーカスするとビューポートでその組が強調されることを書いた
- [x] 文言 (ボタン・欄の名前) がエディタの英語の文言 (`apps/editor/src/lib/i18n`) と合っている。画像は撮り直さない (撮るなら T0968 に書く)
- [x] `just ci` が通る

## 参考

- T1649 のメモ (やらなかったこと・マニュアルの更新の候補)、T1651 のメモ (マニュアルの更新の候補)
- docs/manual/tools.md・tutorial.md・dress-up.md、docs/editor.md (インスペクタの表・グルーの画面)

## メモ

- T0052 で起票した (元の候補: T1649 (tools.md・tutorial.md)・T1651 (dress-up.md))。マニュアルの文の書き足しなので 1 つにまとめた。
- 逆向きの強調 (ビューポートの組 → パネルの行) は T1664 が入れ、そのマニュアルも T1664 で書く。
- やったこと (2026-10-06):
  - tools.md の回転のギズモの節: 「Origin X (this cell)」「Origin Y (this cell)」の欄 (親の座標系。親がワープなら 0〜1、それ以外はピクセル)、
    「Origin: all cells」でラベルが「(all cells)」になり今のセルとの差をすべてのセルに足すこと、チェックがデフォーマを選び直しても残ること、
    キーの間では角度・拡大率・原点の欄が空で押せないこと (all cells でも同じ) を書いた。
  - tutorial.md の 3 章の中心を置き直す囲みに、「Origin: all cells」を付けて「Origin X/Y (all cells)」に数を入れても置き直せることを書き、
    tools.md#回転のギズモ へリンクした。
  - dress-up.md のグルーの節の「Pairs」の説明の後に、行 (と「×」) にポインタ・フォーカスを置くとビューポートでその組が強調されること
    (表示だけで Undo に残らない) を書いた。
  - 文言は apps/editor/locales/en.json (`originX`・`originXAll`・「Origin: all cells」・「Pairs」) と docs/editor.md に合わせた。
  - `just ci` は前面で通った (終了コード 0)。
- やらなかったこと: 画像の撮り直し (T0968 が行う。tools-rotation.png の説明文は今の画像のままにした)。逆向きの強調は T1664。
- 起票の候補: なし。
