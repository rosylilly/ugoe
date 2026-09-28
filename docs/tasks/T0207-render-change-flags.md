---
id: T0207
title: 参照レンダラ: 変更フラグによる転送の省略
status: in-progress
milestone: M2
crates: [ugoe-render]
depends_on: [T0205]
assignee: claude-worker-T0207
---

# T0207: 参照レンダラ: 変更フラグによる転送の省略

## 目的

T0106 の変更フラグ (`ModelInstance::changes()`) を使い、頂点位置やユニフォームが変わっていない描画可能要素の転送を省く。

## 受け入れ条件

- [ ] 1 つの `Renderer` で同じモデルの複数のインスタンスを描く場合や、`encode` の間に `update` が複数回呼ばれる場合でも正しく描画される (インスタンスごとのバッファを持つ、または最後に転送した状態を記録するなど)。T0205 のメモにある懸念を解消する
- [ ] 変わらないフレームで転送量が減ることを確認するテストがある (`FrameStats` に転送量を追加するなど)
- [ ] `just ci` と `just test-gpu` が通る

## 参考

- `docs/spec/evaluation.md#8-出力`
- `docs/tasks/T0106-change-flags.md`、`docs/tasks/T0205-render-mask-semantics.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
