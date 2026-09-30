---
id: T0653
title: エディタ: モジュールのプロジェクトの仕上げ (見た目を保つ移動の物理・本体のバンドルの作り直し・書き出したものとの比較)
status: todo
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0651]
assignee:
---

# T0653: エディタ: モジュールのプロジェクトの仕上げ

## 目的

T0651 の後続の候補をまとめる。

## 受け入れ条件

- [ ] 見た目を保つ移動 (`SetModuleSettings.keep_look`) で、物理の粒子と衝突体の位置も新しい取り付け先の座標に写し直す
- [ ] プレビューの比較で使う本体だけのバンドルを、本体が変わったときだけ作る (今はプレビュー中の編集のたびに作り直す)
- [ ] 前に書き出した同じ名前の `.ugoemod` と、今の編集を並べて比べられる
- [ ] T0712 の見本のプロジェクトで、物理のリグの ▲▼ とテンプレートの無効化を画面で確かめる。プレビュー中の編集で今の編集を取り直す経路も画面で確かめる
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0651-editor-module-project-leftovers.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
