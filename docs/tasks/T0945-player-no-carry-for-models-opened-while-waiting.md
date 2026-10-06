---
id: T0945
title: "@ugoe/player: 作り直しを待つ間に開いたモデルに、前のモデルのパラメータの値を戻さない"
status: todo
milestone: M9
crates: [packages/player, docs]
depends_on: []
assignee:
---

# T0945: @ugoe/player: 作り直しを待つ間に開いたモデルに、前のモデルのパラメータの値を戻さない

## 目的

`Player` は GPU を失って WebPuppet を作り直すとき、前のパラメータの値 (`#carry`) を覚えて作り直した後に戻す。ところが作り直しを待つ間に
利用者が別のモデルを開くと、前のモデルの値が同じ ID のパラメータに戻される (T0942 の気づいたこと・候補。バグ)。
T0942 の利用者の判断 (案 B) では、作り直しを待つ間に開いたモデルのビューは fit (戻さない) にした。値も同じく既定から始める。

## 受け入れ条件

- [ ] 先に失敗するテスト (`player.test.ts`: 作り直しを待つ間に別のモデルを開くと、同じ ID のパラメータが既定値で始まる) を書いてから直した
- [ ] 同じモデルの作り直し (待つ間に何も開かない) では今までどおり値が戻る (既存のテストが通る)
- [ ] docs/ui.md#webpuppet-の寿命 と packages/player/README.md の作り直しの説明を直した
- [ ] `just ci` が通る (`just player-browser` の `player-loss-restore` なども通る)

## 参考

- T0942 のメモ (判断・気づいたこと・起票の候補)
- packages/player/src/player.svelte.ts (`#carry`)

## メモ

- T0040 で起票した (元の候補: T0942 (`#carry`))。
