---
id: T0491
title: ugoe-ffi を std の無い環境向けの静的ライブラリとしてビルドする
status: todo
milestone: M11
crates: [ugoe-ffi, Cargo.toml, Cargo.lock, justfile, scripts, .github/workflows, docs]
depends_on: [T0183, T0184, T0489]
assignee:
---

# T0491: ugoe-ffi を std の無い環境向けの静的ライブラリとしてビルドする

## 目的

ゲーム機や組み込み機器のように Rust の `std` が無い環境に、C ABI を静的ライブラリとして組み込めるようにする
([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md#std-の無い環境への-c-abi))。

## 受け入れ条件

- [ ] `ugoe-ffi` が既定で有効な feature `std` を持ち、外すと `#![no_std]` でビルドできる
- [ ] `std` を外した形のアロケータ (`#[global_allocator]`) は、ホストの C ライブラリの `malloc`・`free` (と整列の要る確保の関数) を呼ぶ。
      パニックの処理 (`#[panic_handler]`) は `abort` を呼ぶ。どちらも SDK ごとの知識を持たない
- [ ] `std` の有無で関数の集合とヘッダが変わらない (エラーは T0489 の `UgoeError` で返すので、スレッドローカルは要らない)。
      `std` を外した形でパニックの処理が `abort` になることを README に書いた
- [ ] `std` を外した形で使えない関数 (テクスチャのデコードなど) をヘッダと README に書いた
- [ ] `aarch64-unknown-none` で staticlib をビルドし、C の見本とリンクして、未解決のシンボルがホストの C ライブラリのもの (`malloc`・`free`・
      `abort` など) だけであることを確かめる検査を足し、CI で回す (実機での確認は対象外。メモに書く)
- [ ] docs/portability.md のターゲット別の表と C ABI の規約、crates/ugoe-ffi/README.md に、std の無い形のビルドと組み込みの手順を書いた
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、crates/ugoe-ffi/README.md、docs/portability.md#c-abi-ugoe-ffi-の規約

## メモ

- 2026-10-06 起票。`panic=abort` は Cargo のプロファイルで決まるので、ワークスペースのプロファイルを変えずに済む方法 (専用のプロファイル) を選ぶ。
- T0180・T0183 からの申し送り (2026-10-07): ugoe-ffi を no_std にするとき、ugoe-ffi と ugoe-format にも clippy.toml (std の数学関数の disallowed-methods) を足し、scripts/nostd-check.mjs の MATH_CRATES に入れる。`ugoe-ffi/src/lighting.rs` の `f32::sqrt` は `ugoe_core::math::sqrt` にする。
