---
id: T1628
title: "エディタ: カーブの区間の種類・フェード・ドープシートのキーの削除も送る直前のカーブから組み立てる"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1628
---

# T1628: エディタ: カーブの区間の種類・フェード・ドープシートのキーの削除も送る直前のカーブから組み立てる

## 目的

T1623 で、ドープシートとカーブのグラフのキー・取っ手のドラッグは、`doc.send` で送る直前のカーブ (`doc.motions`) と `keyMoves` から組み立てるようになった。
ところが `CurveGraph` の区間の種類のボタン (`setKind`)・カーブのフェード (`setFade`)・ドープシートの選んだキーの削除 (`removeSelectedKey`) は、
今も表示 (選択) の時刻から組み立てて `doc.apply` で送る。時刻の移動を送っている間に押すと、古い時刻のキーを指しうる (T1623 の候補)。

## 受け入れ条件

- [x] `setKind`・`setFade`・`removeSelectedKey` が `doc.send` と `keyMoves` で、送る直前のカーブの今の時刻のキーから組み立てる。キーが無くなっていれば送らない
- [x] 先に失敗するテスト (時刻の移動を送っている間に区間の種類を変える・キーを消す) を `CurveGraph.test.ts`・`MotionEditor.test.ts` に入れてから直した
- [x] ダブルクリックのキーの追加も同じ扱いが要るか確かめ、要るなら直し、要らないなら理由をメモに書いた
- [x] docs/editor.md の「タイムライン」の項を直した
- [x] `just ci` が通る

## 参考

- T1623 のメモ (やらなかったこと・起票の候補)
- apps/editor/src/lib/timeline (`timeline.ts` の `KeyMoves`・`CurveGraph`・`MotionEditor`)

## メモ

- T0038 で起票した (元の候補: T1623)。
- やったこと:
  - `CurveGraph` の `setKind`: `moves.track(doc.send(...))` で、送る直前のカーブ (`doc.motions`) で選択の時刻から
    `keyMoves.follow` をたどった今の時刻のキーで終わる区間に `set_motion_segment` を送る。キー・区間が無い (最初のキー)・
    既に同じ種類なら送らない。
  - `CurveGraph` の `setFade`: `doc.send` で、送る直前のカーブのフェードから組み立てる (続けて確定したもう一方のフェードを
    古い表示の値で戻さない)。カーブが無い・値が同じなら送らない。フェードはキーの時刻に依らないので `keyMoves` は使わない。
  - `MotionEditor` の `removeSelectedKey` (「Delete key」と Delete キー): `moves.track(doc.send(...))` で今の時刻のキーを
    `remove_motion_key` で消す。キーが無ければ送らない。消せたら、選択が消したキー (選択の時刻か消した時刻) を指していれば外す
    (送っている間に別のキーを選び直していれば残す)。
  - `keyMoves` は送っている編集が無くなると忘れるので、時刻をたどる `setKind`・`removeSelectedKey` の送信も `track` に渡す
    (渡さないと、前の移動が終わった時点で忘れ、後に並べた編集が古い時刻を探して送らなくなる。テストで確かめた)。
  - テスト (先に失敗することを確かめてから直した): `CurveGraph.test.ts` (送っている間の区間の種類・キーが無くなった後の
    区間の種類・続けてのフェードの確定)、`MotionEditor.test.ts` (送っている間の選んだキーの削除・同じキーの 2 度目の削除)。
  - docs/editor.md の「タイムライン」に「選んだキーへの編集」と、ダブルクリックなどを `doc.apply` のままにする理由を書き、
    「パネルからの編集」の `keyMoves` を共有する操作に区間の種類と削除を足した。
- ダブルクリックのキーの追加は直さなかった: 表示で押した位置の時刻と値に `set_motion_key` を打つだけで、既にあるキーの
  時刻を指さない (行き先は送った時刻の移動と無関係に決まる)。`doc.apply` も同じ列に並ぶので順は保たれる。行の「◆」・
  「◆ Key changed」も同じ (再生位置とスライダーの値)。
- やらなかったこと・確かめていないこと:
  - 実機 (`just editor`) では確かめていない (この環境では画面を操作できない)。happy-dom の部品のテストで確かめた。
  - 「Delete curve」(`remove_motion_curve`) は行の対象だけを指すので `doc.apply` のまま。
- 起票の候補: なし。
- just ci: 前面で実行し終了コード 0 (エディタの Vitest は 76 ファイル・612 件)。
