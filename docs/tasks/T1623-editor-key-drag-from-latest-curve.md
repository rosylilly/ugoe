---
id: T1623
title: "エディタ: ドープシートとカーブのグラフのキー・取っ手のドラッグを、送る直前のカーブから組み立てる"
status: todo
milestone: M9
crates: [apps/editor]
depends_on: []
assignee:
---

# T1623: エディタ: ドープシートとカーブのグラフのキー・取っ手のドラッグを、送る直前のカーブから組み立てる

## 目的

T1612 で、`SlotsPanel`・`CurveGraph` の欄・`GlueOverlay` の編集は、`doc.send` で送る直前の状態からコマンドを組み立てるようにした。
ドープシート (`apps/editor/src/lib/panels/timeline/MotionEditor.svelte`) とカーブのグラフ (`CurveGraph.svelte`) のキー・取っ手のドラッグは、
離したとき (読み直す前) の時刻からコマンドを組み立てる。前の移動を送っている間に同じキーをもう一度ドラッグすると、古い時刻のキーを
動かそうとして失敗しうる (T1612 の候補)。送った時刻の変更をたどる `followKey` (`timeline.ts`) を使って、送る直前のカーブから組み立てる。

## 受け入れ条件

- [ ] ドープシートのキーのドラッグと、カーブのグラフのキー・取っ手のドラッグが、`doc.send` で送る直前のカーブ (`doc.motions`) から組み立て、前の移動を送っている間に続けてドラッグしても今のキーを動かす
- [ ] 先に失敗するテストを書いてから直した (`CurveGraph.test.ts` と、ドープシートの部品のテスト)
- [ ] docs/editor.md#パネルからの編集 の記述と食い違わない (要れば直す)
- [ ] `just ci` が通る

## 参考

- T1612 のメモ (やらなかったこと・起票の候補)
- apps/editor/src/lib/panels/timeline/timeline.ts (`followKey`)、docs/editor.md#パネルからの編集

## メモ

- T0036 で起票した (元の候補: T1612)。
