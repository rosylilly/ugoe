---
id: T0103
title: ポーズとエフェクト設定のデータ定義
status: in-progress
milestone: M1
crates: [ugoe-model]
depends_on: []
assignee: claude-worker-T0103
---

# T0103: ポーズとエフェクト設定のデータ定義

## 目的

ポーズ (排他的なパーツ切り替え) と、目パチ・呼吸・リップシンクの対象パラメータの設定を `ugoe-model` にデータとして追加する (インターフェースを先に入れるタスク)。

## 受け入れ条件

- [ ] `ugoe_model::pose` と `ugoe_model::effects` (仮称) モジュールがあり、serde で往復できるテストがある
- [ ] docs/spec/model.md と docs/spec/motion.md を更新する
- [ ] アルゴリズムは実装しない (T0303 で行う)
- [ ] `just ci` が通る

## 参考

- `docs/spec/motion.md#エフェクト`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
