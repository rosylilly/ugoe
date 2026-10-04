---
id: T0168
title: 着せ替えのエラーの補足に、検証のエラーのコードの一覧と、取り除いた物理の元のコードを載せる (判断が要る)
status: todo
milestone: M9
crates: [ugoe, ugoe-wasm, ugoe-ffi, bindings/web, docs]
depends_on: [T0167]
assignee:
---

# T0168: 着せ替えのエラーの補足に、検証のエラーのコードの一覧と、取り除いた物理の元のコードを載せる (判断が要る)

## 目的

- SDK・wasm・C ABI の `DressUpError::fields` の `errors` は検証のエラーの数だけで、どの検証で落ちたか (`ValidationError::code`) が分からない (T0163 の候補)。
- wasm の `carry_over` が出す `PhysicsRemoved` の警告に、元の `DressUpError` のコードが載らない (理由は英語の文だけ。T0015 の候補)。

**判断が要る:** 補足の値 (`FieldValue`) は今は数と文字列だけ。コードの一覧を載せるには一覧の形 (配列、または `;` でつないだ文字列) を足すことになり、
C ABI (`ugoe_last_error_field`)・wasm・Unity の `UgoeException.Fields`・`@ugoe/runtime` の型に影響する。

## 受け入れ条件

- [ ] 一覧の載せ方を決めた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [ ] `invalid_composition` などの着せ替えのエラーで、検証のエラーのコードの一覧が補足の値に入るテストがある (SDK・wasm・C ABI)
- [ ] wasm の `PhysicsRemoved` に元のコードが補足の値として入るテストがある
- [ ] `crates/ugoe-wasm/codes.json`・`bindings/web` の型・各 README を更新した
- [ ] `just ci`・`just wasm-check`・`just conformance` が通る

## 参考

- T0163・T0015 のメモ (起票の候補)、T0157 (`FieldValue`)
- crates/ugoe/src/dress_up.rs、crates/ugoe-wasm/src/load.rs

## メモ

- T0026 で起票した (元の候補: T0163 の `DressUpError::fields`、T0015 の `carry_over`)。`DressUpError` のコードを使う T0167 の後に着手する。
