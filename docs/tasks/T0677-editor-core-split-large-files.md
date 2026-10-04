---
id: T0677
title: ugoe-editor-core の 1000 行を超えるファイルを分ける
status: done
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: [T0667]
assignee: claude-worker-T0677
---

# T0677: ugoe-editor-core の 1000 行を超えるファイルを分ける

## 目的

T0667 のレビューで、範囲の 4 つのファイルが 1000 行を超えていた (`document.rs` 約 1660 行、`module_compare.rs` 約 1570 行、`project.rs` 約 1160 行、
`module_project.rs` 約 1050 行)。どれも中に独立した部分を持つので、振る舞いを変えずにモジュールに分け、読む範囲を狭くする。

分け方の案 (T0667 の所見):

- `document.rs`: `EditError` (約 160 行)、削除の連鎖 (`remove_parameter`・`remove_part`・`remove_deformer`・`remove_mesh`。
  `docs/editor.md#削除の連鎖と拒否` と対応)、テクスチャと法線マップのコマンド (`add_texture`・`set_texture_image`・`remove_texture`・
  `set_normal_map`。`texture.rs` の隣)、左右の対 (`pair`・`set_mirror_pair`) を別のファイルにする。`Document` と `apply_to` の振り分けだけを残す。
- `module_compare.rs`: アトラス化したファイルを戻す部分 (`FileView`・`Finder`・`Rgba`、約 400 行) を `module_compare/atlas.rs` に、
  要素ごとの中身の違い (`part_changes`〜`mesh_changes`) を `module_compare/changes.rs` に分ける。
- `project.rs`: モジュールのプロジェクトの読み書き (`load_module`・`module_files`・`load_base*`・`read_module_link`) とパスの補助
  (`resolve_base`・`normalize_path`・`relative_path`)、ファイルの書き込みの補助 (`write_if_changed`・`remove_stale`・`list_files`) を分ける。
- `module_project.rs`: `Splitter` (約 330 行) を `module_project/split.rs` に分ける。

## 受け入れ条件

- [x] 4 つのファイルがそれぞれ 1000 行以下になる
- [x] 公開 API (`lib.rs` の `pub use` と `pub mod` の中の公開の項目) が変わらない (`just gen` の生成物に差分がない)
- [x] テストを変えずに通る (移した関数のテストの置き場所を変えるのはよい)
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- T0667 のメモ

## メモ

- やったこと (振る舞いを変えない移動だけ。ファイルごとにコミットを分けた):
  - `document.rs` 1615 → 964 行。`document/` に `error.rs` (`EditError`)・`lookup.rs` (`find_mut`・`ensure_new`・`ensure_no_children`・
    要素を ID で引く補助)・`pair.rs` (`set_mirror_pair`・`pair`)・`remove.rs` (削除の連鎖)・`reorder.rs` (`reorder_mesh`)・
    `texture_edit.rs` (テクスチャと法線マップのコマンド) を分けた。案の 4 つだけでは約 1000 行で残るので、`lookup.rs` と `reorder.rs` も分けた。
    `EditError` は `document.rs` の `pub use` で `crate::document::EditError` のまま。
  - `module_compare.rs` 1569 → 737 行。`module_compare/atlas.rs` (`FileView`・`Finder`・`Rgba`) と `module_compare/changes.rs`
    (`part_changes`〜`mesh_changes` と補助) に分けた。
  - `project.rs` 1163 → 815 行。`project/module.rs` (`load_module`・`module_files`・`load_base*`・`read_module_link`・`BaseCheck`)・
    `project/paths.rs` (`resolve_base`・`normalize_path`・`relative_path`・`absolute`)・`project/files.rs` (`write_if_changed`・
    `remove_stale`・`list_files`・JSON の読み書き・`io_error`) に分けた。公開の項目は `project.rs` の `pub use` で同じパスのまま。
  - `module_project.rs` 1051 → 698 行。`Splitter` と、それだけが使う `ModuleContext::renames` を `module_project/split.rs` に移した。
- 確かめたこと: `just gen` の生成物に差分なし。テストは変えていない (465 件が通る)。`message_tests.rs` が `include_str!("project.rs")` で
  コードを集めるが、移した部分にコードの文字列が無いことを確かめた (集まるコードは変わらない)。`cargo doc` の警告は移す前からのもの
  (`overflow.rs`・`proposal.rs`・`module_project.rs` のモジュールの doc) だけで、移した部分に壊れたリンクはない。`just ci` が通る。
- やらなかったこと: テストのファイル (`tests.rs`・`inference_tests.rs`・`module_project_tests.rs`・`deformer_edit_tests.rs` も 1000 行を超える) は範囲外として分けていない。
- 起票の候補: テストのファイルで 1000 行を超えるもの (`inference_tests.rs` 1800・`tests.rs` 1731・`module_project_tests.rs` 1381・
  `deformer_edit_tests.rs` 1010) を対象の機能ごとに分ける。
