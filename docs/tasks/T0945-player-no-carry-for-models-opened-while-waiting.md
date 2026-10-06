---
id: T0945
title: "@ugoe/player: 作り直しを待つ間に開いたモデルに、前のモデルのパラメータの値を戻さない"
status: done
milestone: M9
crates: [packages/player, docs]
depends_on: []
assignee: claude-worker-T0945
---

# T0945: @ugoe/player: 作り直しを待つ間に開いたモデルに、前のモデルのパラメータの値を戻さない

## 目的

`Player` は GPU を失って WebPuppet を作り直すとき、前のパラメータの値 (`#carry`) を覚えて作り直した後に戻す。ところが作り直しを待つ間に
利用者が別のモデルを開くと、前のモデルの値が同じ ID のパラメータに戻される (T0942 の気づいたこと・候補。バグ)。
T0942 の利用者の判断 (案 B) では、作り直しを待つ間に開いたモデルのビューは fit (戻さない) にした。値も同じく既定から始める。

## 受け入れ条件

- [x] 先に失敗するテスト (`player.test.ts`: 作り直しを待つ間に別のモデルを開くと、同じ ID のパラメータが既定値で始まる) を書いてから直した
- [x] 同じモデルの作り直し (待つ間に何も開かない) では今までどおり値が戻る (既存のテストが通る)
- [x] docs/ui.md#webpuppet-の寿命 と packages/player/README.md の作り直しの説明を直した
- [x] `just ci` が通る (`just player-browser` の `player-loss-restore` なども通る)

## 参考

- T0942 のメモ (判断・気づいたこと・起票の候補)
- packages/player/src/player.svelte.ts (`#carry`)

## メモ

- T0040 で起票した (元の候補: T0942 (`#carry`))。

### やったこと

- `Player.#setUp`: 作り直しを待つ間 (WebPuppet が無い) に利用者がモデルを開いたとき (`#openedWhileDown`) は、`lost` で控えた値
  (`#carry`) を戻さない (捨てる)。ビューの fit (T0942) と同じ条件。同じモデルの作り直しでは今までどおり戻す。
- テスト (player.test.ts): 先に「作り直しを待つ間に別のモデルを開くと、同じ ID のパラメータが既定値で始まる」を書いて落ちる
  (0.5 が戻る) のを確かめてから直した。同じモデルの作り直しで値が戻ることの単体のテストも足した (これまではブラウザのテスト
  `player-loss-restore` だけが確かめていた)。
- 文書: docs/ui.md#webpuppet-の寿命 (Player の項)、packages/player/README.md (作り直しの説明)、`#carry`・`#setUp` のコメント。

### 検査

- `git rebase main` の後に `just ci` (前面、終了コード 0) と `just player-browser` (19 のシナリオがすべて ok。`player-loss-restore`
  を含む)。ランタイム層は触っていない。

### やらなかったこと

- ブラウザのテストに「待つ間に別のモデルを開く」シナリオは足していない (単体のテストで足りると判断。偽の WebPuppet と本物で
  `set_parameter` の振る舞いの差が効く箇所ではない)。

### 起票の候補

- なし。
