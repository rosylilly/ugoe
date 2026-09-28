---
id: T0601
title: エディタのコマンド群
status: in-progress
milestone: M2
crates: [ugoe-editor-core]
depends_on: [T0102]
assignee: claude-worker-T0601
---

# T0601: エディタのコマンド群

## 目的

パーツ・デフォーマ・メッシュ・キーフォームを追加・削除・変更するコマンドを実装する。

## 受け入れ条件

- [ ] 各コマンドに、成功・失敗・Undo のテストがある
- [ ] パラメータの削除時に、そのパラメータを参照するキーフォームの軸も削除する
- [ ] コマンドの適用後のモデルが常に `validate` を通る
- [ ] `just ci` が通る

## 参考

- `docs/editor.md#コマンド`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
