---
id: T0604
title: プロジェクトの保存と読み込み
status: todo
milestone: M2
crates: [ugoe-editor-core, apps/editor (src-tauri のみ)]
depends_on: [T0105]
assignee:
---

# T0604: プロジェクトの保存と読み込み

## 目的

docs/spec/file-formats.md#プロジェクト-ugoeproj の形式で保存と読み込みを行い、`.ugoe` として書き出す。

## 受け入れ条件

- [ ] 保存して読み込むと同じモデルに戻るテストがある
- [ ] 保存を繰り返しても差分が出ない (出力が安定している)
- [ ] エディタのメニューから開く・保存・書き出しができる
- [ ] `just ci` が通る

## 参考

- `docs/spec/file-formats.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
