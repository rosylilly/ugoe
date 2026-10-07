---
id: T0496
title: "ugoe-wasm: std の数学関数の禁止 (MATH_CRATES の clippy.toml) を有効にする"
status: done
milestone: M11
crates: [ugoe-wasm, scripts, docs]
depends_on: []
assignee: claude-worker-T0496
---

# T0496: ugoe-wasm: std の数学関数の禁止 (MATH_CRATES の clippy.toml) を有効にする

## 目的

T0180 で、ランタイム層のクレート (`ugoe-core`〜`ugoe`・`ugoe-format`・`ugoe-ffi`) は std の f32・f64 の数学関数を clippy の
`disallowed_methods` で禁じ、`ugoe_core::math` (常に libm) を通すようにした (scripts/nostd-check.mjs の `MATH_CRATES`)。T0493 で `ugoe-wasm` に
パニックの lint を有効にしたとき、`clippy.toml` (数学関数の禁止も持つ) は写さなかった。`ugoe-wasm` の数学関数も `ugoe_core::math` に寄せ、
`MATH_CRATES` に入れて、ブラウザとエディタ・ほかのホストで計算の結果をそろえる。

T0068 で起票したときに、`crates/ugoe-ffi/clippy.toml` を一時的に `crates/ugoe-wasm/` に写して clippy を流すと、出たのはテストの中の
`f32::sin_cos` の 2 か所 (`src/gizmo.rs`・`src/physics_view.rs` の `mod tests`) だけだった (`--all-targets`。ライブラリのコードは
ホストと `wasm32-unknown-unknown` の `--all-features` で出なかった)。ライブラリの結果は変わらない見込みなので、判断は要らない
(ライブラリのコードで std の数学関数が見つかり、表示の値が変わるなら、メモに書いて進めてよい。公開の値の意味は変わらない)。

## 受け入れ条件

- [x] `crates/ugoe-wasm/clippy.toml` を MATH_CRATES のほかのクレートと同じ内容で置き、`ugoe-wasm` を `MATH_CRATES` に足した (`just groups` の検査が通る)
- [x] 出た箇所を `ugoe_core::math` に直した (テストの中も。clippy.toml はテストのコードにも効く)。ライブラリのコードで直した箇所があれば、
      値が変わるかをメモに書いた
- [x] `just wasm-check` と、`ugoe-wasm` の clippy (ホスト・`wasm32-unknown-unknown` の `--all-features`) が通る
- [x] docs/portability.md (数学関数の対象) と crates/ugoe-wasm/README.md の「守ること」に書いた
- [x] `just ci` が通る

## 参考

- T0493 のメモ (起票の候補 (a))、scripts/nostd-check.mjs (`MATH_CRATES`)、crates/ugoe-ffi/clippy.toml
- docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md#数学関数は-libm-に寄せる

## メモ

- T0068 で起票した (元の候補: T0493 (a) (`ugoe-wasm` の std の数学関数を `ugoe_core::math` に寄せ、`MATH_CRATES` に入れるか決める))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- T0496 (claude-worker-T0496): `crates/ugoe-ffi/clippy.toml` と同じ内容の `crates/ugoe-wasm/clippy.toml` を置き、`ugoe-wasm` を
  `MATH_CRATES` に足した (`just groups`: clippy.toml of 8 crates)。出たのは T0068 の見込みどおりテストの中の `f32::sin_cos` の 2 か所
  (`src/gizmo.rs`・`src/physics_view.rs`) だけで、`ugoe::core::math::sin_cos` に直した。ライブラリのコードは直しておらず、値は変わらない。
  一時的に 1 か所を戻して clippy が `disallowed method f32::sin_cos` で失敗することを確かめた。
  ホストと `wasm32-unknown-unknown` の `--all-features --all-targets` の clippy (`-D warnings`)・`just wasm-check`・`just nostd-check`・
  `just conformance`・`just ci` が通る。
  README の「守ること」の「テストのコードはルートの `clippy.toml` で除く」は、ugoe-wasm 自身の clippy.toml (ルートの設定を写したもの) に書き直した。
- 起票の候補: なし。
