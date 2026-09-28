---
id: T0502
title: 自動メッシュ生成
status: in-progress
milestone: M2
crates: [ugoe-mesher]
depends_on: []
assignee: claude-worker-T0502
---

# T0502: 自動メッシュ生成

## 目的

docs/ai-integration.md#自動メッシュ のパイプラインを実装し、暫定の外接矩形の実装を置き換える。

## 受け入れ条件

- [ ] 穴のある形状、複数の島がある形状を扱える
- [ ] すべての不透明な画素がメッシュに覆われていることを検証するテストがある
- [ ] 頂点数が u16 の範囲に収まる
- [ ] `just ci` が通る

## 参考

- `docs/ai-integration.md#自動メッシュ`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
