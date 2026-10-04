---
id: T0679
title: SetMeshProperties のテクスチャを名前で指定する
status: done
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

- [x] `set_mesh_properties` でテクスチャを名前で指定でき、存在しない名前は `NotFound` で拒否する
- [x] `RemoveTexture` の後に、前の状態で作ったコマンドを適用しても別のテクスチャを指さない (テストで確かめる)
- [x] エディタ・MCP・AI の提案の送り手を合わせて直し、`just gen` の生成物を更新した
- [x] `just ci` が通る

## 参考

- docs/editor.md#コマンド
- docs/adr/0005-editor-command-model.md
- T0667 のメモ

## メモ

- **やったこと:** `Command::SetMeshProperties` に `texture_name` (`TextureRef::name`、大文字小文字まで一致) を足した。ない名前は `NotFound { kind: "texture" }`。
  - **互換の判断:** 番号の `texture` は消さずに非推奨として残し、読んだらそのまま番号を設定する (古い形で書かれたコマンドを読めるように。
    `.ugoeproj` に提案は保存されておらず、エディタ・MCP の提案はドキュメントの状態に結び付いて古くなれば拒否されるので、残る危険は外のスクリプトなどが
    古い形で送る場合だけ)。番号は `#[schemars(skip)]` で AI・MCP のツール定義 (`command.schema.json`) から外し、TS の型には doc コメント付きで残した。
    両方を指定したら新しいエラー `EditError::ConflictingTexture` (`edit.conflicting_texture`、引数 `id`。en/ja の文と `docs/editor.md` の表を追加) で拒否する。
    方針は `docs/editor.md#コマンド一覧` に書いた。
  - エディタのインスペクタ (`MeshProperties.svelte`) はテクスチャを名前で送る。AI (`module_rig/pose.rs`) と CLI のサンプルは `texture` を使っていなかった (フィールドを足しただけ)。
    MCP は `command_schema` から作るので、`just gen` で更新した。
  - テスト: `tests/mesh.rs` (名前での指定・ない名前・大文字小文字違い・番号の読み込み・両方の指定)、`project_tests.rs` の
    `texture_by_name_survives_removing_an_earlier_texture` (前のテクスチャを消した後に、前の状態で作った名前のコマンドが同じテクスチャを指し、番号の形はずれる)。
- **範囲外の変更:** `crates/ugoe-cli/src/sample.rs` (フィールドの追加に合わせて `texture_name: None` を足しただけ)。
- **やらなかったこと:** 番号の `texture` の削除 (将来、古い形を読む必要がなくなったら消せる)。
- **起票の候補:** なし。
