---
id: T0603
title: Rust から TS 型と JSON Schema を生成する
status: done
milestone: M2
crates: [ugoe-editor-core, ugoe-model, apps/editor]
depends_on: []
assignee: claude-worker-T0603
---

# T0603: Rust から TS 型と JSON Schema を生成する

## 目的

ts-rs で TypeScript の型を、schemars で `Command` の JSON Schema を生成し、`ipc.ts` の手書きの型を置き換える。

## 受け入れ条件

- [x] `just gen` で生成でき、CI で生成物が最新かどうかを検査する
- [x] 生成した JSON Schema を `ugoe-ai` から参照できる
- [x] `just ci` が通る

## 参考

- `docs/editor.md#フロントエンドの規約`

## メモ

- 道具は ts-rs 12 (TypeScript) と schemars 1 (JSON Schema)。どちらも MIT で、serde の属性を読む。
  手順・生成物の置き場所・型の対応は `docs/editor.md#型の生成`。
- 生成物はコミットする: `apps/editor/src/lib/generated/{types,samples}.ts` と
  `crates/ugoe-editor-core/schema/command.schema.json`。`just gen` (`apps/editor/src-tauri/examples/gen.rs`) で作り直す。
  最新かどうかは `just test` (src-tauri の `generated_files_are_up_to_date` と editor-core の
  `checked_in_command_schema_is_up_to_date`) と CI の `just gen && git diff --exit-code` で検査する。
- `samples.ts` はフィクスチャのモデルと全バリアントのコマンドを serde で書き出した値で、`svelte-check` が
  生成した型に合うかを検査する。コマンドを追加したら `codegen.rs` の `sample_commands` に例を足す
  (足さないと `samples_cover_every_command_variant` が失敗する)。
- `Id<T>` の `TS` / `JsonSchema` は孤児規則のため `ugoe-core` に置く必要があり、`crates:` の範囲外の
  `ugoe-core` も変更した (feature `ts` / `schema` の裏。ランタイムの依存には入らない)。
- `ugoe-ai` は変更していない。`ugoe_editor_core::codegen::COMMAND_SCHEMA_JSON` / `command_schema()` で参照できる。
- 生成した TS は Biome の対象から外した (`biome.json`)。
- 後続の候補: AI のツール定義に使うとき、`Batch` の再帰や `add_texture` の `png` (数値の配列) を
  スキーマから外すかどうかを決める。
