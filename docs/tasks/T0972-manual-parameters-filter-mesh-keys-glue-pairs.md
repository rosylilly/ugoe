---
id: T0972
title: "マニュアル: パラメータの絞り込みと開閉・メッシュ編集の Shift + ] [ と I・グルーの「Pairs」の一覧を書き足す"
status: in-progress
milestone: M10
crates: [docs]
depends_on: []
assignee: claude-worker-T0972
---

# T0972: マニュアル: パラメータの絞り込みと開閉・メッシュ編集の Shift + ] [ と I・グルーの「Pairs」の一覧を書き足す

## 目的

T1646・T1648 で足した操作が、利用者向けのマニュアル (docs/manual) にまだ無い (どちらのタスクも docs/editor.md だけを直した)。

## 受け入れ条件

- [ ] docs/manual/screen.md の「パラメータ」に、一覧の上の絞り込みの欄 (名前か ID で絞る。絞り込み中は閉じたグループも開いて出る。
      一致が無いときの文) と、グループの見出しの開閉が次に開いたときも保たれることを書いた (T1646)
- [ ] docs/manual/tools.md の「メッシュ編集」のキーの表に Shift + ] / [ (2 つ目の頂点を選ぶ) と I・「Add midpoint」(2 つの頂点の中点に足す) を足し、
      「キーボードだけで点を動かすとき」に頂点を足す手順を書いた (T1648)
- [ ] docs/manual/dress-up.md のグルーの節に、パネルの「Pairs」の一覧 (「×」で外す・頂点の番号で「Add pair」。番号はメッシュ編集の `]` `[` の
      「Point 3 of 12」で確かめられる) を書いた (T1648)
- [ ] 書いた操作と文言 (ボタンの名前・読み上げの文) が `apps/editor/locales/en.json` と docs/editor.md に合っている
- [ ] `just ci` が通る

## 参考

- T1646 のメモ (マニュアルの更新の候補)、T1648 のメモ (マニュアルの更新の候補)
- docs/manual/screen.md、docs/manual/tools.md、docs/manual/dress-up.md、docs/editor.md

## メモ

- T0046 で起票した (元の候補: T1646 (screen.md の「パラメータ」)・T1648 (tools.md のキーの表と dress-up.md の「Pairs」))。
- 画像 (module-glue.png・パラメータのパネル) の撮り直しは T0968 で行う。
