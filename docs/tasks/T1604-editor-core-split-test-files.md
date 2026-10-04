---
id: T1604
title: "editor-core: 1000 行を超えるテストのファイルを機能ごとに分ける"
status: done
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

- [x] 4 つのファイルを対象の機能ごとに分け、どれも 1000 行未満にした (共通の補助は 1 か所に)
- [x] テストの数が分ける前と同じ (`cargo test -p ugoe-editor-core -- --list` の件数をメモに書いた)
- [x] `just ci` が通る

## 参考

- T0677 のメモ (起票の候補)

## メモ

- T0019 で起票した (元の候補: T0677)。ほかの editor-core のタスクと衝突しやすいので、着手の前に並行中のタスクを確かめる。
- やったこと (テストの中身は変えない移動だけ。元のファイルごとにコミットを分けた): 4 つのファイルを、元のファイルを親のモジュールにした
  ディレクトリのモジュールに分けた。共通の補助 (`use`・補助の関数・定数) は親のファイルに残し、子は `use super::*;` で使う。
  子のファイルの先頭の節の見出し (`// ---- … ----`) はモジュールの doc (`//!`) に置き換え、2 つ目からの見出しはそのまま残した。
  - `inference_tests.rs` 1800 → 185 行。`infer.rs` (`InferKeyforms`・`MarkKeyform*`・`SetKeyformInference`)・`mirror.rs`
    (`MirrorKeyforms`・`SetMirrorPair`)・`rederive.rs` (自動の作り直し・由来ごとの作り直し・`RederiveWarning`)・`reparent.rs`
    (見た目を保つ付け替えとミラー)。複数の子で使う `mirror_model`・`mirrored_model`・`right_cell`・`right_origin`・`warnings_of`・`face_grid` は親に置いた。
  - `tests.rs` 1731 → 129 行。`parameter.rs`・`part.rs`・`deformer.rs`・`mesh.rs`・`batch.rs`・`json.rs`。ほかのテストが
    `crate::tests::{applied, rejected, binding, cell}` で使う補助は親のまま (パスは変わらない)。
  - `module_project_tests.rs` 1610 → 143 行 (T1602 の 2 件を含む)。`workspace.rs`・`saving.rs`・`module_settings.rs`・`pose.rs`・
    `colliders.rs`・`rebuild.rs` (T1602)。`TempDir`・`save_base`・`settings` は親に置いた。
  - `deformer_edit_tests.rs` 1010 → 167 行。`create.rs`・`reparent.rs`・`rejection.rs`・`rotation_origin.rs`・`unwrapping.rs`・
    `blend_shape.rs`。`unwrap` (ブレンドシェイプの節でも使う) は親に置いた。
  - いまの最大は `create.rs` 449 行。editor-core の `*_tests` で 1000 行を超えるものはない。
- 確かめたこと: `cargo test -p ugoe-editor-core -- --list` の件数は分ける前も後も 484 件 (ライブラリのテスト 478 件と統合テストなど)。
  テストのパスは `inference_tests::infer::…` のように子のモジュールが挟まるが、「最上位のモジュール::関数名」の集合は前後で一致する
  (並べて diff して差分なし)。空行と見出しを除くコードの行の集合も前後で一致することをスクリプトで確かめた。`message_tests.rs` の
  `include_str!` はソース (`message_codes.rs`・`import.rs`・`inference.rs`・`project.rs`) だけを読み、テストのファイルは読まないので
  集まるコードは変わらない。`just ci` が通る。
- やらなかったこと: テストの中身・名前は変えていない。1000 行に近いソース (`document.rs` 999・`codegen.rs` 987・`deformer_edit.rs` 979) は範囲外。
