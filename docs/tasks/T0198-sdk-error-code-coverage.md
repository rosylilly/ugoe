---
id: T0198
title: "SDK の誤りと警告の種類ごとの見本を SDK の中の網羅の match で作り、wasm・C ABI の写しの検査が新しい種類に気づけるようにする"
status: todo
milestone: M9
crates: [ugoe, ugoe-wasm, ugoe-ffi, docs]
depends_on: []
assignee:
---

# T0198: SDK の誤りと警告のコードの網羅

## 目的

T0197 で SDK の誤りと警告の enum に `#[non_exhaustive]` を付けたため、`ugoe-wasm` の `codes.rs` の `guard` は、SDK の種類が増えたことを網羅の match で気づけなくなった (クレートの外から網羅できない)。網羅の検査を SDK の中に移し、wasm (と C ABI の誤りのコードの写しがあればそれも) が使う。

## 受け入れ条件

- [ ] `ugoe` (か、enum を持つクレート) に、誤りと警告の種類ごとに見本を 1 つずつ返す関数を置いた (`#[doc(hidden)]`。網羅の match で書くので、種類を足すとコンパイルが落ちて見本の追加に気づく)
- [ ] `ugoe-wasm` の `codes.rs` の `guard` (と、写しがあれば C ABI の側) が、この見本をすべてコードに変換できることをテストで確かめる形にした (知らない種類のコードの扱いも)
- [ ] T0197 で足した「SDK に種類を足したら見本も足す」のコメントを、新しい形に合わせて直した
- [ ] `just ci`・`just wasm-check`・`just nostd-check`・`just conformance` が通る

## 参考

- T0197 のメモ (弱くなった点)、crates/ugoe-wasm/src/codes.rs、ADR 0029 (誤りのコード)

## メモ

- 元の候補: T0197。オーケストレーターが起票した (2026-10-12)。
