---
id: T0017
title: crates.io に出すクレートで doc コメントの不足を lint で止める (missing_docs)
status: done
milestone: M9
crates: [ugoe-core, ugoe-model, ugoe-runtime, ugoe-physics, ugoe-motion, ugoe-render, ugoe-format, scripts, docs]
depends_on: []
assignee: claude-worker-T0017
---

# T0017: crates.io に出すクレートで doc コメントの不足を lint で止める (missing_docs)

## 目的

M9 のコードレビューの横断の所見 (T0139。まとめは docs/workflow/code-review.md#m9-の結果)。AGENTS.md は「公開 API には doc コメントを付ける」と決めているが、
検査は無く、レビューで doc コメントの不足が何度も見つかった。T0139 で `ugoe` クレートに `#![warn(missing_docs)]` を入れた (CI の clippy は `-D warnings`)。
crates.io に公開するほかのクレート (`scripts/crates.mjs` の `PUBLISHED`) にも入れる。

## 受け入れ条件

- [x] `PUBLISHED` のクレートすべてに `#![warn(missing_docs)]` を入れ、出た不足を埋めた (中身の無い doc で埋めない)
- [x] `scripts/crates.mjs check` (`just publish-check`) が、`PUBLISHED` のクレートの `lib.rs` に `missing_docs` があることを確かめる
- [x] docs/release.md#cratesio に書いた
- [x] `just ci` と `just wasm-check` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0139 のメモの「横断」、docs/adr/0027-crates-io-publishing.md

## メモ

- やったこと:
  - `PUBLISHED` の 7 クレート (`ugoe-core`・`model`・`runtime`・`format`・`physics`・`motion`・`render`) の `lib.rs` に
    `#![warn(missing_docs)]` を入れ、出た約 490 件 (大半は `ugoe-model`: 検証エラーの variant とフィールド・物理・標準パラメータ・
    モーション) に doc コメントを足した。`ugoe-physics`・`ugoe-motion` は不足が無かった。コードの振る舞いは変えていない
    (1 行で書いていた enum の variant を、フィールドに doc を付けるため複数行にしただけ)。
  - `scripts/crates.mjs` の `problems` が、公開するクレートの lib のソース (cargo metadata の lib ターゲットの `src_path`) に
    `#![warn|deny|forbid(... missing_docs ...)]` があるかを確かめる。テストは `scripts/crates.test.mjs`。
  - docs/release.md#cratesio に検査と、doc コメントを足したら `just gen` を流すことを書いた。docs/workflow/code-review.md の M9 の表も更新。
  - `ugoe-model` の型の doc コメントは生成物に入るので、`just gen` で作り直した (`apps/editor/src/lib/generated/types.ts`・
    `crates/ugoe-editor-core/schema/command.schema.json`・`spec/schema/*.json`)。`crates:` の外だが生成物なので同じ PR に入れた。
    justfile の `publish-check` のコメントにも `missing_docs` を足した (範囲外の小さな変更)。
- 注意 (公開仕様のスキーマ): schemars は doc コメントの付いた unit の variant を `oneOf` の `const` 1 つずつに分けるので、
  `spec/schema/` のいくつかの enum が `{"enum": [...]}` から `oneOf` の `const` の並びに変わった。意味は同じで、
  既に doc のあった enum は元からこの形。生成器 (`spec_files.rs`) で `enum` にまとめ直す案も試したが、既存の形も変わる
  うえ振る舞いの変更になるので入れなかった。
- やらなかったこと: `ugoe-model` の `deformer.rs:54` の rustdoc の警告 (`[0,1]²` が intra-doc link と解釈される。既存で、CI の対象外)。
- 起票の候補:
  - 公開仕様の JSON Schema で、文字列の enum の形 (`enum` か `oneOf` の `const`) を doc コメントの有無に依らず揃える
    (`spec_files.rs` の変換で。公開仕様の生成物が変わるので仕様の担当の判断で)。
  - `cargo doc` の警告 (`rustdoc::broken_intra_doc_links` など) を CI で止める。

