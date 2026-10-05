---
id: T1623
title: "エディタ: ドープシートとカーブのグラフのキー・取っ手のドラッグを、送る直前のカーブから組み立てる"
status: done
milestone: M9
crates: [apps/editor]
depends_on: []
assignee: claude-worker-T1623
---

# T1623: エディタ: ドープシートとカーブのグラフのキー・取っ手のドラッグを、送る直前のカーブから組み立てる

## 目的

T1612 で、`SlotsPanel`・`CurveGraph` の欄・`GlueOverlay` の編集は、`doc.send` で送る直前の状態からコマンドを組み立てるようにした。
ドープシート (`apps/editor/src/lib/panels/timeline/MotionEditor.svelte`) とカーブのグラフ (`CurveGraph.svelte`) のキー・取っ手のドラッグは、
離したとき (読み直す前) の時刻からコマンドを組み立てる。前の移動を送っている間に同じキーをもう一度ドラッグすると、古い時刻のキーを
動かそうとして失敗しうる (T1612 の候補)。送った時刻の変更をたどる `followKey` (`timeline.ts`) を使って、送る直前のカーブから組み立てる。

## 受け入れ条件

- [x] ドープシートのキーのドラッグと、カーブのグラフのキー・取っ手のドラッグが、`doc.send` で送る直前のカーブ (`doc.motions`) から組み立て、前の移動を送っている間に続けてドラッグしても今のキーを動かす
- [x] 先に失敗するテストを書いてから直した (`CurveGraph.test.ts` と、ドープシートの部品のテスト)
- [x] docs/editor.md#パネルからの編集 の記述と食い違わない (要れば直す)
- [x] `just ci` が通る

## 参考

- T1612 のメモ (やらなかったこと・起票の候補)
- apps/editor/src/lib/panels/timeline/timeline.ts (`followKey`)、docs/editor.md#パネルからの編集

## メモ

- T0036 で起票した (元の候補: T1612)。
- やったこと:
  - `timeline.ts` に `KeyMoves` を足した (送ったキーの時刻の変更をモーションと行ごとに覚え、`followKey` でたどる。
    `track(doc.send(...))` で送っている数を数え、すべて終わったら忘れる)。`TimelineState.keyMoves` に 1 つ持たせ、
    ドープシートとカーブのグラフが共有する (ドープシートで動かしたキーを、送っている間にグラフで続けてドラッグしても今のキーを動かす)。
  - `CurveGraph`: キー・取っ手のドラッグを離したら `doc.send` で送る直前のカーブ (`doc.motions`) から組み立てる。キーは
    今の時刻のキーを、表示で離した位置へ動かす (時刻を動かさなかったドラッグ (Shift・縦だけ) は今の時刻のまま。隣を越えるなら
    `timeline.keyBlocked` を出して送らない)。取っ手は今の時刻のキーで終わる区間の取っ手を、今の区間の中に収めて送る
    (区間がベジェでなくなっていれば送らない)。押して動かさずに離したときは送らない。欄の確定 (T1612) も `keyMoves` に移した
    (部品ごとの `moved`・`sending` をやめた)。
  - `MotionEditor` のドープシート: キーのドラッグを `doc.send` で送る直前のカーブから組み立てる (今の時刻のキーを、表示で離した
    時刻へ。値は今のキーの値)。使わなくなった `moveKey` を消した。
  - テスト (先に失敗することを確かめてから直した): `CurveGraph.test.ts` (送っている間のキーの続けてのドラッグ・動かしたキーの
    取っ手のドラッグ・欄の確定の後のドラッグ)、新しい `MotionEditor.test.ts` (ドープシートの続けてのドラッグ・ドープシートの
    後のグラフのドラッグ)、`timeline.test.ts` (`KeyMoves`)。`MotionEditor` を描くため、テストの道具の `FakeDocument` に
    `parameters` を足した。happy-dom は幅を測れないので、`MotionEditor.test.ts` は `clientWidth` を 600 にする。
  - docs/editor.md の「パネルからの編集」と「タイムライン」のドラッグの項に、送る直前のカーブから組み立てることと `keyMoves` を書いた。
- やらなかったこと・確かめていないこと:
  - 実機 (`just editor`) では確かめていない (この環境では画面を操作できない)。happy-dom の部品のテストで確かめた。
  - 行き先は「表示で離した位置」(絶対) にした。送っている間に表示が古いまま離した場合も、読み直した後に見える位置は離した位置になる。
  - ダブルクリックのキーの追加・区間の種類のボタン・カーブのフェードは、今も表示 (選択) の時刻から組み立てる (`doc.apply`)。
    区間の種類のボタンは、送っている時刻の移動の後に押すと古い時刻を指しうる。
- 起票の候補:
  - `CurveGraph` の区間の種類のボタン (`setKind`)・カーブのフェード (`setFade`)・ドープシートの選んだキーの削除
    (`removeSelectedKey`) も `doc.send` と `keyMoves` で送る直前のカーブから組み立てる。
- just ci: 前面で実行し終了コード 0 (エディタの Vitest は 71 ファイル・576 件)。その後 `git rebase main` (取り込んだのは
  ビューア・Unity・タスクファイルの変更だけ) し、エディタの Vitest をもう一度通した。
