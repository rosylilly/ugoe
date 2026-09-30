---
id: T0119
title: 公開仕様: JSON Schema・例のファイル・同期の検査
status: done
milestone: M5
crates: [spec, ugoe-model, ugoe-format, scripts, justfile, .github/workflows, docs, package.json, pnpm-lock.yaml]
depends_on: [T0116]
assignee: claude-worker-T0119
---

# T0119: 公開仕様: JSON Schema・例のファイル・同期の検査

## 目的

公開仕様に JSON Schema と例のファイル (適合性を確かめるためのモデルと期待する評価結果) を付け、日本語の開発用の仕様書と英語の公開仕様の食い違いを CI で見つけられるようにする。

## 受け入れ条件

- [x] `spec/schema/` に `model.json` などの JSON Schema を生成して置く (`just gen` で作り、差分がないことを CI で確かめる)。`$id` とバージョン
- [x] `spec/examples/`: 小さなモデルと、指定したパラメータの値での頂点の位置・不透明度などの期待値。ugoe のランタイムがそれを満たすテスト
- [x] `docs/spec/` と `spec/` の同期の検査 (例: 両方の見出し・フィールドの一覧・機能の表の対応) を `just` のレシピと CI に入れる
- [x] `just ci` が通る

## 参考

- `docs/adr/0021-public-specification.md`
- `crates/ugoe-editor-core/src/codegen.rs` (型の生成)

## メモ

### やったこと

- **JSON Schema:** `ugoe_model::spec_files` (feature `schema`) が `Model`・`Module`・`PhysicsSettings`・`Motion`・`Expression` の
  スキーマ (draft 2020-12) を `spec/schema/*.schema.json` に生成する。`$id` (リポジトリの raw URL)、`title` に仕様の版 (0.1 Draft)、
  `$comment` に CC BY 4.0、`schema_version` は `const: 0`。doc コメント (日本語) は載せない。既定値の f32 は最短の 10 進で書く。
  ファイル名は `model.json` (モデルの文書の慣例の名前) と紛れないよう `*.schema.json` にした。
- **例:** `spec/examples/<name>/model.json` (フィクスチャ 7 つから生成) と、手で導いた `expected.json`。
  テストは `crates/ugoe-format/tests/spec_examples.rs` (JSON で読み、検証し、ランタイムで評価して比べる)。
  許容誤差は T0117 と合わせた評価の適合の基準 (位置 0.01 px、不透明度・色 1e-4、draw_order 1e-3、同順位は順不同)。
- **生成:** `just gen` に `cargo run -p ugoe-model --features schema --example spec_files` の 1 行を足した。
  最新かどうかは `ugoe-model` の `tests/spec_files.rs` と CI の既存の `just gen && git diff --exit-code`。
- **同期の検査:** `just spec` (`check-md-links.mjs docs spec` と `check-spec.mjs`) を `just ci` と CI の `web` ジョブに入れた。
  内容は `docs/spec/public-spec.md#同期の検査` (例のスキーマ検証 (ajv)、`spec/model.md` の型の表と JSON Schema、機能 ID の表と
  `features.rs`、未知の値の表、対応表が両方の `##` 見出しを覆うこと、`spec/README.md` の文書の一覧)。
  Markdown の道具は `scripts/lib/markdown.mjs` に切り出し、`check-md-links.mjs` もそれを使う。
- 今の文書を通すため、対応表に「表の行にない節」(日本語版だけの節の一覧) と「JSON Schema と例」の節を足した。

### 範囲外の変更 (team-lead の承認済み)

- `crates/ugoe-format/tests/spec_examples.rs`: 例をランタイムで評価するテスト (ugoe-format は既に ugoe-runtime を dev-dep に持つ)。
- `package.json`・`pnpm-lock.yaml`: devDependency に ajv (MIT。`pnpm add` で更新)。`check-spec.mjs` が例をスキーマで検証する。
- `ugoe-model` の `serde_json` は feature `schema` の optional (既定とランタイムのビルドには入らない。`just wasm-check` 済み)。

### 確かめていないこと・残した課題

- スキーマは書き手の形だけを表す (未知の値は拒み、未知のフィールドは許す)。`spec/schema/README.md` に明記した。
- `$id` は Draft の間は main の raw URL。1.0 でタグの URL などに固定する。
- 例は評価 (ランタイム) だけ。物理・モーション・描画の例と、モジュール・物理などの単体 JSON の例はない (後続の候補)。
  `check-spec.mjs` は `module.json`・`*.physics.json` などを置けば名前でスキーマを選んで検証する。
- `check-spec.mjs` の型の表の読み取りは `spec/model.md` の書き方 (「`` `Type`: ``」と `| Field |` の表など) に依存する。
  書き方を変えると検査が落ちるので、そのときは検査を直す。
- T0117 のマージ後に rebase し (team-lead の依頼)、`spec/rendering.md`・`physics.md`・`motion.md` の Overview・Unknown values・
  Determinism の行を対応表に足した。`spec/evaluation.md#conformance` から `spec/examples/README.md` へリンクし、
  例の許容誤差はその節 (英語版が正) と同じ値にした。描画・物理・モーションの Conformance からは、例がないのでリンクしていない。
