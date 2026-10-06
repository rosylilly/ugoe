---
id: T0572
title: "MeshError::InvalidOptions の理由をフィールドと値にして automesh.invalid_options の reason を訳す"
status: done
milestone: M9
crates: [ugoe-mesher, ugoe-editor-core, apps/editor, docs]
depends_on: [T0571]
assignee: claude-worker-T0572
---

# T0572: MeshError::InvalidOptions の理由をフィールドと値にして automesh.invalid_options の reason を訳す

## 目的

T0558 で、自動メッシュの失敗は `automesh.*` のコードを持つようになったが、`MeshError::InvalidOptions` の理由 (`MeshOptions::validate` の文) は
英語のまま、`automesh.invalid_options`・`psd.invalid_mesh_options` の `reason` に入る (T0558 の候補)。

## 受け入れ条件

- [x] `MeshError::InvalidOptions` が、どのフィールドがどの値でどの範囲を外れたか (例: `{ field, value, min, max }` や理由の種類) を持つ。ugoe-mesher のテストで確かめる
- [x] editor-core の `Coded for MeshError` と `psd.invalid_mesh_options` がフィールドの名前と値を引数で出し、エディタの日英のカタログで訳す (`sourced_codes` のテストと見本を直した)
- [x] crates/ugoe-mesher/README.md・docs/editor.md を直した
- [x] `just ci` が通る

## 参考

- T0558 のメモ (起票の候補)
- crates/ugoe-mesher (`MeshOptions::validate`・`MeshError`)

## メモ

- T0038 で起票した (元の候補: T0558 (`MeshError::InvalidOptions`))。
- やったこと (T0572):
  - `ugoe-mesher` に `MeshOption` (`Margin`・`EdgeSpacing`・`InteriorSpacing`。`#[non_exhaustive]`) を足し、
    `MeshError::InvalidOptions { option, value }` にした。受け付ける範囲は `MeshOption::bounds()` (`std::ops::Bound` の組) で、
    `validate` はこれ 1 か所で判定する (有限であることも要る)。`range_text()` は区間の記法 (`[0, 1024]`・`(0, inf)`・`[0, inf)`)。
    フィールドが f32 になったので `MeshError` は `Eq` を外した (`PartialEq` は残す)。
  - editor-core: `AutoMeshOptions::check` は `Result<(), MeshError>` を返す。`PsdImportError::InvalidMeshOptions { option, value }`。
    `automesh.invalid_options`・`psd.invalid_mesh_options` の引数は `option`・`value`・`range` (`reason` をやめた)。日英のカタログ・
    `messageCodes.ts` (`just gen`)・見本 (`message_catalog.rs`)・テスト (`import_tests`・`automesh` のテスト) を直した。
  - `option` (フィールドの名前) と `range` (区間の記法) は訳さずに渡す。エディタの画面に自動メッシュの設定のラベルが無く、利用者が
    書くのは JSON のフィールド名なので、名前のままのほうが分かる。`errors.ts` の `IDENTIFIER_ARGS` には足していない (apps/editor は T0673 が並行)。
  - crates/ugoe-mesher/README.md・docs/editor.md (取り込みの節とコードの一覧) を直した。
- やらなかったこと: 範囲の上限・下限を別の数の引数 (`min`・`max`) にはしなかった (`edge_spacing` の下限は含まず上限も無いので、
  1 つの文で訳せる区間の記法にまとめた)。
- 起票の候補: なし。
