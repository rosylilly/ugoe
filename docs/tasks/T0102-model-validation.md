---
id: T0102
title: モデル検証の完成
status: in-progress
milestone: M1
crates: [ugoe-model]
depends_on: []
assignee: claude-worker-T0102
---

# T0102: モデル検証の完成

## 目的

`ugoe_model::validate` に、docs/spec/model.md#検証 で ★ が付いている検査項目を実装する。

## 受け入れ条件

- [ ] ★ の項目がすべて実装され、項目ごとに失敗するケースのテストがある
- [ ] エラーは最初の 1 件で止めずに、すべて返す
- [ ] docs/spec/model.md の ★ を消す
- [ ] `just ci` が通る

## 参考

- `docs/spec/model.md#検証`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
