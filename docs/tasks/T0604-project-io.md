---
id: T0604
title: プロジェクトの保存と読み込み
status: done
milestone: M2
crates: [ugoe-editor-core, apps/editor (src-tauri のみ)]
depends_on: [T0105]
assignee: claude-worker-T0604
---

# T0604: プロジェクトの保存と読み込み

## 目的

docs/spec/file-formats.md#プロジェクト-ugoeproj の形式で保存と読み込みを行い、`.ugoe` として書き出す。

## 受け入れ条件

- [x] 保存して読み込むと同じモデルに戻るテストがある
- [x] 保存を繰り返しても差分が出ない (出力が安定している)
- [ ] エディタのメニューから開く・保存・書き出しができる
- [x] `just ci` が通る

## 参考

- `docs/spec/file-formats.md`

## メモ

- `ugoe_editor_core::project` に `Project` (`save` / `load`) と `export_bundle` を追加した。形式の詳細は
  `docs/spec/file-formats.md#プロジェクト-ugoeproj`、設計は `docs/editor.md#プロジェクト`。
- `Document` がテクスチャの画像 (`Textures` = 名前 → PNG、`Arc<[u8]>`) を持ち、Undo の対象にした。
  コマンド `add_texture` / `set_texture_image` / `remove_texture` を追加した (T0605 の PSD 取り込みで使う)。
- `Document::is_modified` / `mark_saved` で未保存の変更を追う (Undo で保存時に戻ると未変更に戻る)。
- Tauri: `src-tauri/src/project.rs` に `project_status` `new_project` `open_project` `save_project`
  `save_project_as` `export_bundle`。物理・モーション・表情は編集コマンドがないので `Document` の外
  (`ProjectSession`) に置いている。
- フロントエンド: 指示によりタスクの範囲 (src-tauri のみ) の外の `src/lib/FileMenu.svelte` を追加し、
  `App.svelte` のヘッダに置いた。`ipc.ts` も更新した。
- **実行時に確かめていないこと:** GUI は起動して試していない (ビルドと型検査、Rust のテストだけ)。
  そのため「メニューから開く・保存・書き出しができる」は未確認として残した。
- 後続の候補: ファイルダイアログ (`tauri-plugin-dialog`) の導入とライセンス確認、物理・モーション・
  表情の編集コマンドと `Document` への移動、`sources/` への PSD のコピー (T0605)、
  ビューポートへの反映 (新規・開くの後に T0402 のバンドル再読み込みを呼ぶ)。
