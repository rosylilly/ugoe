---
id: T0457
title: C ABI のモデルの読み込みの失敗にも種類を付けて wasm の LoadError と揃え、wasm の着せ替えの GPU の喪失にコードを付ける
status: in-progress
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, docs]
depends_on: []
assignee: claude-worker-T0457
---

# T0457: C ABI のモデルの読み込みの失敗にも種類を付けて wasm の LoadError と揃え、wasm の着せ替えの GPU の喪失にコードを付ける

## 目的

T0434 の残り (T0434 の候補)。

- C ABI のモデルの読み込み (`ugoe_puppet_create_from_*`) の失敗は、上限を超えたときだけ `ugoe_last_error_kind` を持つ。wasm の `LoadError` の
  コード (`invalid_file`・`unsupported_version`・`invalid_model`・`compile` など) と揃える。種類を決める処理は SDK (`ugoe` クレート) に置き、wasm と C ABI で共有する。
- wasm の着せ替えの付け外しで GPU を失ったときの例外 (`with_wardrobe!` の「the GPU device was lost」) にコードが無い。`DressUpError` の `device_lost` を付ける。

## 受け入れ条件

- [ ] 読み込みの失敗の種類を SDK で決め (`LoadError` 相当の `code()`)、wasm と C ABI がそれを使う
- [ ] C ABI の読み込みの失敗ごとに `ugoe_last_error_kind` の値を確かめるテストがあり、`ugoe.h` の説明を更新した (Unity の `UgoeResult`・`Fields` に影響があればメモに)
- [ ] wasm の着せ替えで GPU を失ったときに `name: "DressUpError"`・`code: "device_lost"` になる (ネイティブのテストで)
- [ ] crates/ugoe-wasm/README.md・crates/ugoe-ffi/README.md・`codes.json` を更新した
- [ ] `just ci`・`just wasm-check`・`just conformance` が通る

## 参考

- T0434 のメモ (起票の候補)、crates/ugoe-wasm/README.md#読み込みの失敗-t0434

## メモ

- T0026 で起票した (元の候補: T0434 の C ABI の種類と wasm の `device_lost`)。
