---
id: T0183
title: ugoe (SDK、render を外した形) を no_std + alloc にする
status: done
milestone: M11
crates: [ugoe, Cargo.toml, Cargo.lock, scripts, docs]
depends_on: [T0181, T0182]
assignee: claude-worker-T0183
---

# T0183: ugoe (SDK、render を外した形) を no_std + alloc にする

## 目的

`Puppet` を中心とする SDK を `no_std + alloc` で使えるようにし、ランタイム層の no_std 対応を仕上げる
([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md))。

## 受け入れ条件

- [x] `ugoe` が `#![no_std]` + `extern crate alloc;` で、既定で有効な feature `std` を持つ。`render` の feature は `std` を含む
- [x] `default-features = false` (render も std も無し) で、`Puppet` の読み込み・評価・物理・モーション・トラッキング・着せ替えが使える
- [x] `ugoe` を `just nostd-check` の対象に足した。`just sdk-no-render-check` (wgpu・naga が入らないこと) も今までどおり通る
- [x] docs/portability.md の `ugoe` の feature の表に `std` を足し、「対応中」の注記を、決まった方針の説明に書き換えた
- [x] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、docs/portability.md#ugoe-クレートの-feature、[ADR 0027](../adr/0027-crates-io-publishing.md)

## メモ

- 2026-10-06 起票。
- T0182 からの申し送り (2026-10-07): ugoe は ugoe-format を `default-features = false` で取り、自分の `std` の feature から ugoe-format の `std` (と必要なら `png`) を有効にする。
- 2026-10-07 完了 (claude-worker-T0183)。
  - やったこと: `ugoe` を `#![cfg_attr(not(test), no_std)]` + `extern crate alloc;` にし、`std::` を `core::`・`alloc::` に替えた。
    feature `std` (既定で有効) を足し、下位のクレート (`ugoe-core`・`ugoe-model`・`ugoe-format`・`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`) と
    `thiserror` の `std` をそこから有効にする。依存はすべて `default-features = false` (ワークスペースの側で切ってある) で取る。
    `render` は `std` を含む。`ugoe` に `cfg(feature = "std")` の項目は無く、公開 API は `std` の有無で変わらない。
  - `scripts/nostd-check.mjs` の `NOSTD_CRATES` に `ugoe` を足した (`just nostd-check` は 7 クレート)。
  - docs/portability.md: feature の表に `std` を足し、`default-features = false` で `std` も外れることを書いた。「対応中」の注記を
    `NOSTD_CRATES` と対象外のクレート (`ugoe-render`・`ugoe-wasm`) と `ugoe-ffi` の std の無い形 (T0491) の説明に書き換えた。
    crates/ugoe/README.md の feature と守ることも直した。
  - 数学関数の禁止 (clippy.toml の `disallowed-methods`): `ugoe` は T0180 で `MATH_CRATES` に入っていて `crates/ugoe/clippy.toml` もある。足すものは無い。
  - 使う側の Cargo.toml (`ugoe-ffi`・`bindings/godot`・`ugoe-headless` など。`ugoe` を `default-features = false` で取る) は変えていない。
    `std` を外しても API が同じで、`ugoe-format` の `png` などはそれぞれが自分で取っているので、ビルドもテストも通る (`just godot-check`・`just conformance`)。
  - 確かめたこと: `just nostd-check`・`just wasm-check`・`just sdk-no-render-check`・`just conformance`・`just godot-check`・
    `just publish-check`・`just cross-check`・`just ci` がすべて通った。2 つ目の受け入れ条件は、`ugoe` のライブラリ全体が
    `aarch64-unknown-none` で `--no-default-features` のままビルドできること (`Puppet` の読み込み・評価・物理・モーション・トラッキング・
    着せ替えの API はすべて feature に関係なく入っている) と、`just sdk-no-render-check` の `cargo test -p ugoe --no-default-features` で確かめた。
    実際の `std` の無い機器での実行は確かめていない (T0491 の C の見本とのリンクの検査が近いもの)。
  - 起票の候補:
    - `ugoe-ffi` (T0491 で no_std にするとき) と `ugoe-format` を `MATH_CRATES` に入れ、clippy.toml を置く。`ugoe-ffi` の
      `src/lighting.rs` に `f32::sqrt` の直接の呼び出しが 1 つある (`ugoe_core::math::sqrt` に替える)。`ugoe-format` は今は数学関数を呼んでいない。
