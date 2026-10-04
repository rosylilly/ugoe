---
id: T0017
title: crates.io に出すクレートで doc コメントの不足を lint で止める (missing_docs)
status: todo
milestone: M9
crates: [ugoe-core, ugoe-model, ugoe-runtime, ugoe-physics, ugoe-motion, ugoe-render, ugoe-format, scripts, docs]
depends_on: []
assignee:
---

# T0017: crates.io に出すクレートで doc コメントの不足を lint で止める (missing_docs)

## 目的

M9 のコードレビューの横断の所見 (T0139。まとめは docs/workflow/code-review.md#m9-の結果)。AGENTS.md は「公開 API には doc コメントを付ける」と決めているが、
検査は無く、レビューで doc コメントの不足が何度も見つかった。T0139 で `ugoe` クレートに `#![warn(missing_docs)]` を入れた (CI の clippy は `-D warnings`)。
crates.io に公開するほかのクレート (`scripts/crates.mjs` の `PUBLISHED`) にも入れる。

## 受け入れ条件

- [ ] `PUBLISHED` のクレートすべてに `#![warn(missing_docs)]` を入れ、出た不足を埋めた (中身の無い doc で埋めない)
- [ ] `scripts/crates.mjs check` (`just publish-check`) が、`PUBLISHED` のクレートの `lib.rs` に `missing_docs` があることを確かめる
- [ ] docs/release.md#cratesio に書いた
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0139 のメモの「横断」、docs/adr/0027-crates-io-publishing.md

## メモ
