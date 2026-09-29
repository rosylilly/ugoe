---
id: T0208
title: 参照レンダラ: 描画の自動統合
status: in-progress
milestone: M4
crates: [ugoe-render]
depends_on: [T0205]
assignee: claude-worker-T0208
---

# T0208: 参照レンダラ: 描画の自動統合

## 目的

描画順で隣り合い、描画の状態が同じ要素を 1 回の描画にまとめる。要素ごとの値は頂点属性またはインスタンスのデータで渡す。

## 受け入れ条件

- [ ] 描画順を入れ替えない
- [ ] 統合した結果が、要素ごとに描いた結果と一致する GPU テストがある
- [ ] `FrameStats` に描画回数・状態の切り替え回数・転送バイト数を出す
- [ ] WebGL2 で動く (`just wasm-check`、GL の GPU テスト)
- [ ] `docs/spec/rendering.md#描画の最適化` を更新する
- [ ] `just ci` が通る

## 参考

- `docs/adr/0016-draw-batching.md`
- `docs/spec/rendering.md#描画の最適化`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
