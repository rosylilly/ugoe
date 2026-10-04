---
id: T1604
title: "editor-core: 1000 行を超えるテストのファイルを機能ごとに分ける"
status: in-progress
milestone: M9
crates: [ugoe-editor-core]
depends_on: []
assignee: claude-worker-T1604
---

# T1604: editor-core: 1000 行を超えるテストのファイルを機能ごとに分ける

## 目的

T0677 で `ugoe-editor-core` の大きなソースのファイルを分けた。テストのファイルにも 1000 行を超えるものが 4 つある
(T0677 の時点で `inference_tests.rs` 約 1800 行・`tests.rs` 約 1731 行・`module_project_tests.rs` 約 1381 行・`deformer_edit_tests.rs` 約 1010 行)。
対象の機能ごとに分け、どこに何のテストがあるかを分かりやすくする (振る舞いは変えない)。

## 受け入れ条件

- [ ] 4 つのファイルを対象の機能ごとに分け、どれも 1000 行未満にした (共通の補助は 1 か所に)
- [ ] テストの数が分ける前と同じ (`cargo test -p ugoe-editor-core -- --list` の件数をメモに書いた)
- [ ] `just ci` が通る

## 参考

- T0677 のメモ (起票の候補)

## メモ

- T0019 で起票した (元の候補: T0677)。ほかの editor-core のタスクと衝突しやすいので、着手の前に並行中のタスクを確かめる。
