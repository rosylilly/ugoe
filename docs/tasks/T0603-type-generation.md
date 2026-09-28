---
id: T0603
title: Rust から TS 型と JSON Schema を生成する
status: todo
milestone: M2
crates: [ugoe-editor-core, ugoe-model, apps/editor]
depends_on: []
assignee:
---

# T0603: Rust から TS 型と JSON Schema を生成する

## 目的

ts-rs で TypeScript の型を、schemars で `Command` の JSON Schema を生成し、`ipc.ts` の手書きの型を置き換える。

## 受け入れ条件

- [ ] `just gen` で生成でき、CI で生成物が最新かどうかを検査する
- [ ] 生成した JSON Schema を `ugoe-ai` から参照できる
- [ ] `just ci` が通る

## 参考

- `docs/editor.md#フロントエンドの規約`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
