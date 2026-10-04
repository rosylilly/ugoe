---
id: T0677
title: ugoe-editor-core の 1000 行を超えるファイルを分ける
status: todo
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: [T0667]
assignee:
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

- [ ] 4 つのファイルがそれぞれ 1000 行以下になる
- [ ] 公開 API (`lib.rs` の `pub use` と `pub mod` の中の公開の項目) が変わらない (`just gen` の生成物に差分がない)
- [ ] テストを変えずに通る (移した関数のテストの置き場所を変えるのはよい)
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- T0667 のメモ

## メモ
