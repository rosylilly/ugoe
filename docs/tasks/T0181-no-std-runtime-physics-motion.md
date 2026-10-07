---
id: T0181
title: ugoe-runtime・ugoe-physics・ugoe-motion を no_std + alloc にする
status: done
milestone: M11
crates: [ugoe-runtime, ugoe-physics, ugoe-motion, Cargo.toml, Cargo.lock, scripts, docs]
depends_on: [T0179, T0180]
assignee: claude-worker-T0181
---

# T0181: ugoe-runtime・ugoe-physics・ugoe-motion を no_std + alloc にする

## 目的

評価・物理・モーションのクレートを `no_std + alloc` でビルドできるようにする ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md))。

## 受け入れ条件

- [x] 3 つのクレートが `#![no_std]` + `extern crate alloc;` で、既定で有効な feature `std` を持つ。`std` を外しても公開 API は変わらない
- [x] `HashMap` (`ugoe-runtime`) を `BTreeMap` か決まったハッシュ関数の `hashbrown` に置き換えた (評価の決定性を保つ)
- [x] `ugoe-motion` のトラッキングの `Mutex` (`relayout`) を、no_std で使える形に置き換えた (`Sync` が要るかを確かめ、要らなければ `RefCell`、
      要るなら計算をやり直す形など。スピンロックを持ち込む前に要るかを考える)
- [x] `ModelInstance::id()` のアトミックなカウンタが `aarch64-unknown-none` で使えることを確かめた (64 ビットのアトミックが無いターゲットは対象外とメモに書く)
- [x] 3 つのクレートを `just nostd-check` の対象に足した
- [x] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、docs/portability.md#ランタイム層が守る制約-移植性のため

## メモ

- 2026-10-06 起票。

### やったこと

- 3 つのクレートを `#![cfg_attr(not(test), no_std)]` + `extern crate alloc;` にし (テストだけ std の prelude。`ugoe-format` と同じ)、
  既定で有効な feature `std` を足した (`std` は依存の `std` を有効にするだけで、`cfg(feature = "std")` のコードは無い。公開 API は同じ)。
  `std::` は `core::`・`alloc::` に置き換え、`Vec`・`String`・`vec!`・`format!` などは `use alloc::…;` で明示した。
- ルートの `[workspace.dependencies]` で `ugoe-runtime`・`ugoe-physics`・`ugoe-motion` の既定の feature を切り、std で使うクレート
  (`ugoe`・`ugoe-format` の開発時の依存・`ugoe-headless`・`ugoe-editor-core`・`ugoe-rigging`・`ugoe-cli`・`ugoe-ai`・`ugoe-render`) に
  `features = ["std"]` を書いた (`crates:` の外だが Cargo.toml の 1 行ずつの変更)。`ugoe-runtime` の `glam` は `std` を外した形では
  `ugoe-core` が有効にする `libm` を使う。
- `HashMap`・`HashSet` (`ugoe-runtime`): `IdIndex` (ID → 添字) は `BTreeMap<Box<str>, u32>`、`compose` の重複・所属の検査は
  `BTreeMap`・`BTreeSet` にした。新しい依存は足していない。
  - `hashbrown` + 決まったハッシュ関数 (`foldhash::fast::FixedState`) にしなかった理由: 依存が 2 つ増えるうえ、種の決まったハッシュは
    ファイルから来る ID で衝突を狙われうる (読み込みが O(n²) になる)。`BTreeMap` は決定的で、衝突の心配が無い。
  - 速さ (`cargo run --release -p ugoe-runtime --example id_lookup_timing`): 300 パラメータ + 300 パーツを ID で引く 600 回が
    0.0075 ms → 0.0133 ms (1 回 12 ns → 22 ns)。`update` (0.0074 ms) と同じくらいで、毎フレーム全部を ID で引く呼び出し側でも小さい。
    20 + 20 では差が無い (0.0005 → 0.0004 ms)。
  - 引き当てが「定数時間」でなく O(log n) になったので、`crates/ugoe-runtime/README.md`・`docs/spec/evaluation.md`・doc コメントの
    「定数時間」を「線形探索にしない (O(log n))」に直した (英語の `spec/` には該当の記述が無い)。
- `ugoe-motion` の `TrackingMap` の `Mutex` (`relayout`): `Sync` は要る (T0327 のメモ: `RefCell` だと `TrackingMap` と `Puppet` が
  `Sync` でなくなり、Bevy のコンポーネントなどで困る。テストでも `Send + Sync` を確かめている) ので、`RefCell` にはしなかった。
  スピンロックも持ち込まず、控えをやめて「計算をやり直す」形にした: 作ったときと並びの違う `ParameterValues` を受けたら、
  書き込み先ごとに ID で引き直す (T0327 より前の、控えが取れなかったときと同じ経路)。`Clone` は derive に戻した。
  - 今の呼び出し側 (Puppet・FFI・wasm・Godot・スタジオ) は作ったときと同じ並びに書くので、この経路は通らず、速さは変わらない。
  - テスト `keeps_indices_resolved_for_a_different_layout` (控えの有無を見ていた) を `writes_to_a_different_layout_every_time`
    (違う並びに続けて書いても同じ値を書く・元の並びに戻しても書く・`Send + Sync`) に置き換えた。`ResolvedParams::layout_for_test` は使われなくなったので消した。
  - `crates/ugoe-motion/README.md` の T0327 の説明を直した。
- `ModelInstance::id()` のカウンタ (`core::sync::atomic::AtomicU64`): `aarch64-unknown-none` で clippy が通る (`just nostd-check`) ことを確かめた。
  64 ビットのアトミックを持たないターゲット (`thumbv6m-none-eabi`・`riscv32imc` など) は対象外 (`docs/portability.md` に書いた)。
- `scripts/nostd-check.mjs` の `NOSTD_CRATES` に 3 つを足した。`docs/portability.md` の「対応中」と既定の feature を切った依存の一覧を直した。
- 振る舞いは変えていない: physics の golden (`golden_tests.rs`。T0180 で記録し直したハッシュ) は変えずに通る。
- 通した検査: `just nostd-check`・`just wasm-check`・`just conformance`・`just spec`・`just cross-check`・`just ci`。

### やらなかったこと

- 64 ビットのアトミックの無いターゲットへの対応 (カウンタを `AtomicU32` や `portable-atomic` にするなど)。需要が出たら考える。
- 実機 (ゲーム機・組み込み機器) での確認。

### 起票の候補

- なし。
