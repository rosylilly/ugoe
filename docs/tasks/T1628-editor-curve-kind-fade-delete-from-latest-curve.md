---
id: T1628
title: "エディタ: カーブの区間の種類・フェード・ドープシートのキーの削除も送る直前のカーブから組み立てる"
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee:
---

# T1628: エディタ: カーブの区間の種類・フェード・ドープシートのキーの削除も送る直前のカーブから組み立てる

## 目的

T1623 で、ドープシートとカーブのグラフのキー・取っ手のドラッグは、`doc.send` で送る直前のカーブ (`doc.motions`) と `keyMoves` から組み立てるようになった。
ところが `CurveGraph` の区間の種類のボタン (`setKind`)・カーブのフェード (`setFade`)・ドープシートの選んだキーの削除 (`removeSelectedKey`) は、
今も表示 (選択) の時刻から組み立てて `doc.apply` で送る。時刻の移動を送っている間に押すと、古い時刻のキーを指しうる (T1623 の候補)。

## 受け入れ条件

- [ ] `setKind`・`setFade`・`removeSelectedKey` が `doc.send` と `keyMoves` で、送る直前のカーブの今の時刻のキーから組み立てる。キーが無くなっていれば送らない
- [ ] 先に失敗するテスト (時刻の移動を送っている間に区間の種類を変える・キーを消す) を `CurveGraph.test.ts`・`MotionEditor.test.ts` に入れてから直した
- [ ] ダブルクリックのキーの追加も同じ扱いが要るか確かめ、要るなら直し、要らないなら理由をメモに書いた
- [ ] docs/editor.md の「タイムライン」の項を直した
- [ ] `just ci` が通る

## 参考

- T1623 のメモ (やらなかったこと・起票の候補)
- apps/editor/src/lib/timeline (`timeline.ts` の `KeyMoves`・`CurveGraph`・`MotionEditor`)

## メモ

- T0038 で起票した (元の候補: T1623)。
