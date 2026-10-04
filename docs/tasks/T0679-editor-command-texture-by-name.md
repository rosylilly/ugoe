---
id: T0679
title: SetMeshProperties のテクスチャを名前で指定する
status: in-progress
milestone: M9
crates: [ugoe-editor-core, ugoe-mcp, ugoe-ai, apps/editor, docs]
depends_on: [T0667]
assignee: claude-worker-T0679
---

# T0679: SetMeshProperties のテクスチャを名前で指定する

## 目的

T0667 のレビューの所見。`Command` の規則は「フィールドは ID で対象を指定する (インデックスは使わない)」(`docs/editor.md#コマンド`) だが、
`Command::SetMeshProperties` の `texture` は `Model::textures` の番号 (`u32`) で指定する。`RemoveTexture` は後ろのテクスチャの番号を詰めるので、
AI の提案や MCP のエージェントが古い状態で作った `set_mesh_properties` を後で適用すると、別のテクスチャを指してしまう (検証は通る)。

テクスチャを名前 (`TextureRef::name`) で指定する形に替える。JSON の互換 (既存の提案・MCP のツール定義) をどう扱うか
(新しいフィールドを足して古い番号を拒否する・読み替える) を決め、`docs/editor.md` に書く。

## 受け入れ条件

- [ ] `set_mesh_properties` でテクスチャを名前で指定でき、存在しない名前は `NotFound` で拒否する
- [ ] `RemoveTexture` の後に、前の状態で作ったコマンドを適用しても別のテクスチャを指さない (テストで確かめる)
- [ ] エディタ・MCP・AI の提案の送り手を合わせて直し、`just gen` の生成物を更新した
- [ ] `just ci` が通る

## 参考

- docs/editor.md#コマンド
- docs/adr/0005-editor-command-model.md
- T0667 のメモ

## メモ
