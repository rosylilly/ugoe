---
id: T0303
title: モーションマネージャ・表情・エフェクト
status: in-progress
milestone: M3
crates: [ugoe-motion, ugoe]
depends_on: [T0103, T0302]
assignee: claude-worker-T0303
---

# T0303: モーションマネージャ・表情・エフェクト

## 目的

優先度付きのモーションマネージャ、表情、目パチ・呼吸・リップシンク・ポーズを実装し、`Puppet::update` の適用順を仕様どおりにする。

## 受け入れ条件

- [ ] docs/spec/motion.md の各項目にテストがある
- [ ] フレーム内の適用順の `> 未決:` を解消する
- [ ] `just ci` が通る

## 参考

- `docs/spec/motion.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
