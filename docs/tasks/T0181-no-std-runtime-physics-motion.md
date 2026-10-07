---
id: T0181
title: ugoe-runtime・ugoe-physics・ugoe-motion を no_std + alloc にする
status: in-progress
milestone: M11
crates: [ugoe-runtime, ugoe-physics, ugoe-motion, Cargo.toml, Cargo.lock, scripts, docs]
depends_on: [T0179, T0180]
assignee: claude-worker-T0181
---

# T0181: ugoe-runtime・ugoe-physics・ugoe-motion を no_std + alloc にする

## 目的

評価・物理・モーションのクレートを `no_std + alloc` でビルドできるようにする ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md))。

## 受け入れ条件

- [ ] 3 つのクレートが `#![no_std]` + `extern crate alloc;` で、既定で有効な feature `std` を持つ。`std` を外しても公開 API は変わらない
- [ ] `HashMap` (`ugoe-runtime`) を `BTreeMap` か決まったハッシュ関数の `hashbrown` に置き換えた (評価の決定性を保つ)
- [ ] `ugoe-motion` のトラッキングの `Mutex` (`relayout`) を、no_std で使える形に置き換えた (`Sync` が要るかを確かめ、要らなければ `RefCell`、
      要るなら計算をやり直す形など。スピンロックを持ち込む前に要るかを考える)
- [ ] `ModelInstance::id()` のアトミックなカウンタが `aarch64-unknown-none` で使えることを確かめた (64 ビットのアトミックが無いターゲットは対象外とメモに書く)
- [ ] 3 つのクレートを `just nostd-check` の対象に足した
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、docs/portability.md#ランタイム層が守る制約-移植性のため

## メモ

- 2026-10-06 起票。
