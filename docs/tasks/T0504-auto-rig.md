---
id: T0504
title: 自動リギングの提案
status: todo
milestone: M6
crates: [ugoe-ai]
depends_on: [T0503, T0601, T0509]
assignee:
---

# T0504: 自動リギングの提案

## 目的

意味ラベルとテンプレートから `Proposal` を作る。

## 受け入れ条件

- [ ] 顔の角度・目の開閉・口の開閉の提案が作れる
- [ ] 提案が `validate` を通る
- [ ] 検証エラーを AI にフィードバックして再試行する
- [ ] `just ci` が通る

## 参考

- `docs/ai-integration.md#自動リギング`
- `crates/ugoe-cli/src/sample.rs` (T0702・T0704 の見本のジェネレータ。レイヤーの名前から部位を見つけ、コマンドの列でリグを組む。テンプレートの出発点になる)
- `crates/ugoe-editor-core/src/physics_template.rs` (T0634 の物理のテンプレート)

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
