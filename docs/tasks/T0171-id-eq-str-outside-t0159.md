---
id: T0171
title: T0159 の範囲の外の Id の as_str() == を id == "..." に置き換える
status: done
milestone: M9
crates: [apps/studio, ugoe-format, ugoe-motion, ugoe-physics, ugoe-render, ugoe-wasm, ugoe, tests/conformance, bindings/godot]
depends_on: []
assignee: claude-worker-T0171
---

# T0171: T0159 の範囲の外の Id の as_str() == を id == "..." に置き換える

## 目的

T0159 で `Id<T>` に文字列との比較 (`PartialEq<str>` など) を足し、範囲のクレートの `as_str() == "..."` を `id == "..."` に置き換えた。
範囲の外に 21 か所 (apps/studio 5・ugoe-format の tests 3・ugoe-motion 2・ugoe-physics の tests 3・ugoe-render 2・ugoe-wasm 2・ugoe 3・tests/conformance 1) と、
ワークスペースの外の bindings/godot に 2 か所 (model.rs・node.rs) が残っている (T0159 の候補)。振る舞いは変えない機械的な置き換え。

## 受け入れ条件

- [x] `grep -rn 'as_str() ==' --include=*.rs` で、`Id` の比較が残っていない (文字列の型どうしの比較など、`Id` でないものは残してよい。残したものはメモに)
- [x] bindings/godot の 2 か所も置き換え、README にある Godot の検査 (`node bindings/godot/build.mjs` など) が通る
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0159 のメモ (起票の候補)
- crates/ugoe-core/src/id.rs

## メモ

- T0032 で起票した (元の候補: T0159)。
- やったこと: `as_str` を一時的に `#[deprecated]` にしてワークスペース全体 (`--all-targets`) の警告を集め、`==`・`!=` で比べている行を `id == "..."` に置き換えた (25 か所。godot の 2 か所を含む)。`--features gpu-tests` の `ugoe-render/tests/gpu.rs` も 1 か所。
  - 相手が `String` のときは `Id` と `String` を直接比べられないので、`other.as_str()` (String の方) か `*mesh_id(..)` で `str` にした。`&Id` と `str` は `*part == id` と参照を外した。
  - 起票のときより後に入った範囲外の 4 か所も、機械的な置き換えなので直した: ugoe-ai (`turn.rs` 1・`tests/module_rig_sample.rs` 2)・ugoe-editor-core (`tests/m2/mod.rs` 1)。apps/studio の `animation.rs` (`id == p.as_str()`、`p` は `String`) も逆向きの比較として直した。
- 残したもの (`Id` ではない): `ugoe-ai/src/classify.rs` の `Label::as_str()`、`ugoe-mcp` の `server.rs`・`http.rs` の `serde_json::Value::as_str()`、apps/studio の `api/methods.rs` の `value["slot"] == slot.as_str()` (`slot` は `String`)。`ugoe-editor-core` の `module_compare.rs` の `own(g.id.as_str())` は比較ではない。
- 検査: `just ci`・`just wasm-check`・`just godot-check` (66 件すべて通過) が通った。
