---
id: T0183
title: ugoe (SDK、render を外した形) を no_std + alloc にする
status: todo
milestone: M11
crates: [ugoe, Cargo.toml, Cargo.lock, scripts, docs]
depends_on: [T0181, T0182]
assignee:
---

# T0183: ugoe (SDK、render を外した形) を no_std + alloc にする

## 目的

`Puppet` を中心とする SDK を `no_std + alloc` で使えるようにし、ランタイム層の no_std 対応を仕上げる
([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md))。

## 受け入れ条件

- [ ] `ugoe` が `#![no_std]` + `extern crate alloc;` で、既定で有効な feature `std` を持つ。`render` の feature は `std` を含む
- [ ] `default-features = false` (render も std も無し) で、`Puppet` の読み込み・評価・物理・モーション・トラッキング・着せ替えが使える
- [ ] `ugoe` を `just nostd-check` の対象に足した。`just sdk-no-render-check` (wgpu・naga が入らないこと) も今までどおり通る
- [ ] docs/portability.md の `ugoe` の feature の表に `std` を足し、「提案中」の注記を、決まった方針の説明に書き換えた (ADR 0031 が Accepted になっていれば)
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、docs/portability.md#ugoe-クレートの-feature、[ADR 0027](../adr/0027-crates-io-publishing.md)

## メモ

- 2026-10-06 起票。
