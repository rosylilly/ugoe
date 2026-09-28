---
id: T0611
title: 推定・ミラーのエディタコマンド
status: todo
milestone: M2
crates: [ugoe-editor-core]
depends_on: [T0610, T0601]
assignee:
---

# T0611: 推定・ミラーのエディタコマンド

## 目的

`InferKeyforms` / `MirrorKeyforms` / `MarkKeyformAuthored` / `MarkKeyformDerived` を実装し、元のセルが変わったときに `derived` のセルを自動で作り直す。

## 受け入れ条件

- [ ] 各コマンドに成功・失敗・Undo のテストがある
- [ ] 元のセルを編集するコマンドの後、影響を受ける `derived` のセルが同じ Undo 単位で作り直される
- [ ] `derived` のセルを編集すると `authored` になる
- [ ] `just ci` が通る

## 参考

- `docs/spec/keyform-inference.md#推定の実行`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
