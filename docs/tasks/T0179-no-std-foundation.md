---
id: T0179
title: no_std の土台 (依存の既定の feature・just nostd-check・ugoe-core と ugoe-model)
status: todo
milestone: M11
crates: [ugoe-core, ugoe-model, Cargo.toml, Cargo.lock, justfile, scripts, .github/workflows, docs]
depends_on: []
assignee:
---

# T0179: no_std の土台 (依存の既定の feature・just nostd-check・ugoe-core と ugoe-model)

## 目的

ランタイム層を `no_std + alloc` でビルドできるようにする ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)) ための土台を作る。
検査の仕組みを先に入れ、依存の根元にある `ugoe-core` と `ugoe-model` を no_std にする。後続 (T0180〜T0183) はこの検査に自分のクレートを足していく。

## 受け入れ条件

- [ ] ルートの `[workspace.dependencies]` で、ランタイム層が使う依存 (`glam`・`serde`・`serde_json`・`thiserror`・`ruzstd`・`rmp-serde` など) の
      既定の feature を切り、必要な feature を使う側 (ランタイム層の `std` の feature・ツール層のクレート) に書いた
      (メンバーの `default-features = false` は、ワークスペースの側で切っていないと効かないため)。今のビルドの結果 (有効な feature) が変わらないことを確かめた
- [ ] `ugoe-core` と `ugoe-model` が `#![no_std]` + `extern crate alloc;` で、既定で有効な feature `std` を持つ。`std` を外しても公開 API は変わらない
      (`HashSet` などは `BTreeSet` か決まったハッシュ関数の `hashbrown` に置き換える。エラー型は `core::error::Error`)
- [ ] `just nostd-check`: 対象のクレートを `aarch64-unknown-none` で `--no-default-features` でビルドする (`rustup target add` の案内を含む)。
      対象の一覧は 1 か所 (スクリプト) に置き、後続のタスクが足せる。CI のジョブ (`rust (runtime)` など) で回す
- [ ] `docs/portability.md#ランタイム層が守る制約-移植性のため` に `just nostd-check` と、ランタイム層では `core::`・`alloc::` を使う規則を書いた。
      AGENTS.md のコマンドの表にも足した
- [ ] `just wasm-check`・`just sdk-no-render-check` が今までどおり通る
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、docs/portability.md、docs/testing.md#ci-のジョブ
- `aarch64-unknown-none` は `core` と `alloc` がビルド済みで配られる tier 2 のターゲットで、`std` が無いので漏れはビルドの失敗になる

## メモ

- 2026-10-06、Go (Ebitengine) やゲーム機への組み込みの検討から起票した。ADR 0031 は Proposed。着手の前に利用者が承認する。
