---
id: T0202
title: 参照レンダラ: マスク
status: in-progress
milestone: M1
crates: [ugoe-render]
depends_on: [T0201]
assignee: claude-worker-T0202
---

# T0202: 参照レンダラ: マスク

## 目的

マスクアトラス方式のクリッピング (和集合、反転) を実装する。

## 受け入れ条件

- [ ] 同じマスク集合を共有する要素で、マスクの描画を使い回す
- [ ] 反転マスクのテストがある
- [ ] アトラスが足りない場合に、アトラスを増やして描画できる
- [ ] `just ci` が通る

## 参考

- `docs/spec/rendering.md#マスク`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
