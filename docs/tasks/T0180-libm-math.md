---
id: T0180
title: ランタイム層の数学関数を ugoe_core::math (常に libm) に寄せる
status: todo
milestone: M11
crates: [ugoe-core, ugoe-runtime, ugoe-physics, ugoe-motion, ugoe, Cargo.toml, Cargo.lock, tests/conformance, tools/spec-check, assets/fixtures, docs]
depends_on: [T0179]
assignee:
---

# T0180: ランタイム層の数学関数を ugoe_core::math (常に libm) に寄せる

## 目的

`f32` の `sin`・`cos`・`powf`・`exp`・`atan2` などは `std` にしか無い。ランタイム層が数学関数を `ugoe_core::math` から呼ぶようにし、
その中身は `std` の有無にかかわらず常に `libm` にする ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md#数学関数は-libm-に寄せる))。
no_std で使えるようになるのと同時に、OS の数学ライブラリの違いによる出力の差が無くなる。

## 受け入れ条件

- [ ] `ugoe-core` に `math` モジュールを足し、ランタイム層 (`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe`) の `f32`・`f64` の数学関数の呼び出しを
      すべて置き換えた。`glam` は `libm` の feature を有効にした
- [ ] 置き換えの漏れを検査する仕組みがある (clippy の `disallowed_methods` などで、ランタイム層のクレートで `f32::sin` などを直接呼ぶと警告になる)
- [ ] 適合テスト (`just conformance`)・spec-check・スナップショットが許容誤差の中で通る。収まらないものは理由を確かめてから期待値を作り直した
- [ ] 置き換えの前後で `Puppet::update` の速さを見本のモデルで測り、メモに書いた (目に見えて遅い関数があれば対策した)
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、docs/testing.md、tests/conformance/README.md

## メモ

- 2026-10-06 起票。`sqrt`・`abs` などは IEEE で結果が決まるので、どちらを使っても値は同じ。
