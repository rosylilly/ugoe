---
id: T0654
title: エディタ: 今の編集と書き出したモジュールの違いを文字で出す・本体だけのバンドルをセッション側で持つ
status: in-progress
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0653]
assignee: claude-worker-T0654
---

# T0654: エディタ: モジュールの比較の仕上げ

## 目的

T0653 の後続の候補をまとめる。

## 受け入れ条件

- [ ] プレビューの着せ替えの一覧で、今の編集 (`<名前> (editing)`) と、同じ名前の書き出したファイルの違い (要素の数・テクスチャ・条件など) を文字でも出す
- [ ] 本体だけのバンドルを、フロントエンドではなくセッション (Tauri の側) で持ち回る (今は PreviewMode がバイト列を覚えている)
- [ ] エディタのモジュールの書き出し (Export Module…) でも、アトラス化 (`build_module_atlas`、T0132) を選べるようにする
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0653-editor-module-project-polish.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
