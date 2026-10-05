---
id: T0572
title: "MeshError::InvalidOptions の理由をフィールドと値にして automesh.invalid_options の reason を訳す"
status: todo
milestone: M9
crates: [ugoe-mesher, ugoe-editor-core, apps/editor, docs]
depends_on: [T0571]
assignee:
---

# T0572: MeshError::InvalidOptions の理由をフィールドと値にして automesh.invalid_options の reason を訳す

## 目的

T0558 で、自動メッシュの失敗は `automesh.*` のコードを持つようになったが、`MeshError::InvalidOptions` の理由 (`MeshOptions::validate` の文) は
英語のまま、`automesh.invalid_options`・`psd.invalid_mesh_options` の `reason` に入る (T0558 の候補)。

## 受け入れ条件

- [ ] `MeshError::InvalidOptions` が、どのフィールドがどの値でどの範囲を外れたか (例: `{ field, value, min, max }` や理由の種類) を持つ。ugoe-mesher のテストで確かめる
- [ ] editor-core の `Coded for MeshError` と `psd.invalid_mesh_options` がフィールドの名前と値を引数で出し、エディタの日英のカタログで訳す (`sourced_codes` のテストと見本を直した)
- [ ] crates/ugoe-mesher/README.md・docs/editor.md を直した
- [ ] `just ci` が通る

## 参考

- T0558 のメモ (起票の候補)
- crates/ugoe-mesher (`MeshOptions::validate`・`MeshError`)

## メモ

- T0038 で起票した (元の候補: T0558 (`MeshError::InvalidOptions`))。
