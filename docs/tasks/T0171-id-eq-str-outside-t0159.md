---
id: T0171
title: T0159 の範囲の外の Id の as_str() == を id == "..." に置き換える
status: todo
milestone: M9
crates: [apps/studio, ugoe-format, ugoe-motion, ugoe-physics, ugoe-render, ugoe-wasm, ugoe, tests/conformance, bindings/godot]
depends_on: []
assignee:
---

# T0171: T0159 の範囲の外の Id の as_str() == を id == "..." に置き換える

## 目的

T0159 で `Id<T>` に文字列との比較 (`PartialEq<str>` など) を足し、範囲のクレートの `as_str() == "..."` を `id == "..."` に置き換えた。
範囲の外に 21 か所 (apps/studio 5・ugoe-format の tests 3・ugoe-motion 2・ugoe-physics の tests 3・ugoe-render 2・ugoe-wasm 2・ugoe 3・tests/conformance 1) と、
ワークスペースの外の bindings/godot に 2 か所 (model.rs・node.rs) が残っている (T0159 の候補)。振る舞いは変えない機械的な置き換え。

## 受け入れ条件

- [ ] `grep -rn 'as_str() ==' --include=*.rs` で、`Id` の比較が残っていない (文字列の型どうしの比較など、`Id` でないものは残してよい。残したものはメモに)
- [ ] bindings/godot の 2 か所も置き換え、README にある Godot の検査 (`node bindings/godot/build.mjs` など) が通る
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0159 のメモ (起票の候補)
- crates/ugoe-core/src/id.rs

## メモ

- T0032 で起票した (元の候補: T0159)。
