---
id: T0073
title: "check-licenses.mjs: --locked で失敗したとき、スタックトレースでなく古い lock のファイルと直し方を短く出す"
status: in-progress
milestone: M11
crates: [scripts, docs]
depends_on: []
assignee: claude-worker-T0073
---

# T0073: check-licenses.mjs: --locked で失敗したとき、スタックトレースでなく古い lock のファイルと直し方を短く出す

## 目的

`scripts/check-licenses.mjs` の `--fuzz` (と `--godot`) は `cargo metadata --locked` を使う。ランタイムのクレートに依存が増えて
`fuzz/Cargo.lock` や `bindings/godot/Cargo.lock` が古くなると、Node のスタックトレースが出て、何を直せばよいか分かりにくい (T0072 の候補)。

## 受け入れ条件

- [ ] `--locked` で lock のファイルが古いために失敗したときは、どの lock のファイルが古いかと直し方 (`cargo update --manifest-path <…>/Cargo.toml --workspace`) を短く出し、終了コード 1 で止まる (スタックトレースを出さない)。ほかの失敗は今までどおり詳しく出す
- [ ] スクリプトのテストで、古い lock の場合の文を確かめた
- [ ] docs/testing.md (fuzz・godot の検査) の直し方の説明とそろえた
- [ ] `just ci` が通る

## 参考

- T0072 のメモ、scripts/check-licenses.mjs、fuzz/README.md

## メモ

- T0072 のワーカーの報告から起票した (2026-10-07)。
