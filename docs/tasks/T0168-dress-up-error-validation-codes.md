---
id: T0168
title: 着せ替えのエラーの補足に、検証のエラーのコードの一覧と、取り除いた物理の元のコードを載せる (判断が要る)
status: done
milestone: M9
crates: [ugoe, ugoe-wasm, ugoe-ffi, bindings/web, docs]
depends_on: [T0167]
assignee: claude-worker-T0168
---

# T0168: 着せ替えのエラーの補足に、検証のエラーのコードの一覧と、取り除いた物理の元のコードを載せる (判断が要る)

## 目的

- SDK・wasm・C ABI の `DressUpError::fields` の `errors` は検証のエラーの数だけで、どの検証で落ちたか (`ValidationError::code`) が分からない (T0163 の候補)。
- wasm の `carry_over` が出す `PhysicsRemoved` の警告に、元の `DressUpError` のコードが載らない (理由は英語の文だけ。T0015 の候補)。

**判断が要る:** 補足の値 (`FieldValue`) は今は数と文字列だけ。コードの一覧を載せるには一覧の形 (配列、または `;` でつないだ文字列) を足すことになり、
C ABI (`ugoe_last_error_field`)・wasm・Unity の `UgoeException.Fields`・`@ugoe/runtime` の型に影響する。

## 受け入れ条件

- [x] 一覧の載せ方を決めた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [x] `invalid_composition` などの着せ替えのエラーで、検証のエラーのコードの一覧が補足の値に入るテストがある (SDK・wasm・C ABI)
- [x] wasm の `PhysicsRemoved` に元のコードが補足の値として入るテストがある
- [x] `crates/ugoe-wasm/codes.json`・`bindings/web` の型・各 README を更新した
- [x] `just ci`・`just wasm-check`・`just conformance` が通る

## 参考

- T0163・T0015 のメモ (起票の候補)、T0157 (`FieldValue`)
- crates/ugoe/src/dress_up.rs、crates/ugoe-wasm/src/load.rs

## メモ

- T0026 で起票した (元の候補: T0163 の `DressUpError::fields`、T0015 の `carry_over`)。`DressUpError` のコードを使う T0167 の後に着手する。
- **一覧の載せ方 (判断):** 外に出す値の形は変えず、`, ` でつないだ文字列にした (T0457 の `unsupported_features` の `features` と同じつなぎ方)。
  キーは新しく `error_codes` を足し、既存の `errors` (数) はそのまま残したので、wasm (`string`)・C ABI (`ugoe_last_error_field`)・
  Unity (`Dictionary<string, string>`)・`@ugoe/runtime` (`error_codes?: string`) とも、既存の値の意味は変わらず、キーが 1 つ増えるだけ。
  種類は出た順で、重なりを除く (同じ種類のエラーが多数出ても長くならないように。数は `errors` にある)。配列にする案は TS・Unity の値の型が増えるので採らなかった。
- **SDK の型 (Rust):** `FieldValue` は借用の `Text(&str)` だけでは、つないだ文字列 (持ち主のある値) を返せない。`FieldValue::Codes(Vec<&'static str>)`
  を足した (`Display` は `, ` でつなぐ。`Copy` は外れた)。`Text` を `Cow` にする案もあったが、既存の `Text("..")` の比較をすべて直すことになり、
  一覧であることも型で分からないので採らなかった。版は 0.0.0 で未公開。網羅の `match` をしている Godot のバインディング (`bindings/godot`、`crates:` の外) に
  腕を 1 つ足した (`cargo clippy --manifest-path bindings/godot/Cargo.toml` で確認。Godot 本体での `just godot-check` は流していない)。
- **`physics_removed`:** `LoadWarning::PhysicsRemoved` に `codes: Vec<&'static str>` を足し、理由が検査のエラーなら `errors`・`error_codes` を補足の値に出す
  (`invalid_physics` と同じ形)。調べると wasm の `carry_over` が呼ぶ `Puppet::set_physics` は `Vec<ValidationError>` を返し、`DressUpError` は通らない
  (新しい `Puppet` にはモジュールが付いていない)。元の種類は常に `invalid_physics` 相当で一定なので、種類ではなく検査のエラーの種類の一覧を載せた。
- Unity の `UgoeException.FieldKeys` に `error_codes` を足した (`bindings/unity`、`crates:` の外。codes.json と食い違うと `ResultTests` が落ちるため)。
  `node bindings/unity/build.mjs --check` が通る (dotnet test 93 件)。
- `just ci`・`just wasm-check`・`just refs`・`just conformance` が通る。
- 起票の候補: 読み込みの失敗 `invalid_model` (`ugoe::LoadError::fields`。`errors` の数だけ) にも同じ `error_codes` を載せる。
  C ABI の `ugoe_puppet_set_base_physics` の検査の失敗は種類を持たない (`UGOE_ERROR_INVALID_MODEL`) ので、`invalid_physics` と補足の値を付ける。
