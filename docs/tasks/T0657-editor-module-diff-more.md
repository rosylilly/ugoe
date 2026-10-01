---
id: T0657
title: エディタ: 物理・衝突体・グルーの違いを出す・ビューポートで強調する・.ugoemod にアトラスの対応表
status: todo
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0656]
assignee:
---

# T0657: モジュールの違いの残り

## 目的

T0656 の後続の候補をまとめる。

## 受け入れ条件

- [ ] 物理のリグ・衝突体・グルーの違いを中身まで出し、ツリーやパネルで選べるようにする
- [ ] プレビューを出ずに、ビューポートで違いの要素を強調する
- [ ] .ugoemod にアトラスの対応表を残すかを決める (ファイル形式の変更になるので、残すなら ADR・spec/・docs/spec・SCHEMA_VERSION・互換性の資料)
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0656-editor-module-diff-details.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
