---
id: T0179
title: no_std の土台 (依存の既定の feature・just nostd-check・ugoe-core と ugoe-model)
status: done
milestone: M11
crates: [ugoe-core, ugoe-model, Cargo.toml, Cargo.lock, justfile, scripts, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0179
---

# T0179: no_std の土台 (依存の既定の feature・just nostd-check・ugoe-core と ugoe-model)

## 目的

ランタイム層を `no_std + alloc` でビルドできるようにする ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)) ための土台を作る。
検査の仕組みを先に入れ、依存の根元にある `ugoe-core` と `ugoe-model` を no_std にする。後続 (T0180〜T0183) はこの検査に自分のクレートを足していく。

## 受け入れ条件

- [x] ルートの `[workspace.dependencies]` で、ランタイム層が使う依存 (`glam`・`serde`・`serde_json`・`thiserror`・`ruzstd`・`rmp-serde` など) の
      既定の feature を切り、必要な feature を使う側 (ランタイム層の `std` の feature・ツール層のクレート) に書いた
      (メンバーの `default-features = false` は、ワークスペースの側で切っていないと効かないため)。今のビルドの結果 (有効な feature) が変わらないことを確かめた
- [x] `ugoe-core` と `ugoe-model` が `#![no_std]` + `extern crate alloc;` で、既定で有効な feature `std` を持つ。`std` を外しても公開 API は変わらない
      (`HashSet` などは `BTreeSet` か決まったハッシュ関数の `hashbrown` に置き換える。エラー型は `core::error::Error`)
- [x] `just nostd-check`: 対象のクレートを `aarch64-unknown-none` で `--no-default-features` でビルドする (`rustup target add` の案内を含む)。
      対象の一覧は 1 か所 (スクリプト) に置き、後続のタスクが足せる。CI のジョブ (`rust (runtime)` など) で回す
- [x] `docs/portability.md#ランタイム層が守る制約-移植性のため` に `just nostd-check` と、ランタイム層では `core::`・`alloc::` を使う規則を書いた。
      AGENTS.md のコマンドの表にも足した
- [x] `just wasm-check`・`just sdk-no-render-check` が今までどおり通る
- [x] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、docs/portability.md、docs/testing.md#ci-のジョブ
- `aarch64-unknown-none` は `core` と `alloc` がビルド済みで配られる tier 2 のターゲットで、`std` が無いので漏れはビルドの失敗になる

## メモ

- 2026-10-06、Go (Ebitengine) やゲーム機への組み込みの検討から起票した。ADR 0031 は同日に Accepted。
- 2026-10-07 (claude-worker-T0179) やったこと:
  - `[workspace.dependencies]` の `glam`・`ruzstd`・`serde`・`serde_json`・`thiserror` と、内部の `ugoe-core`・`ugoe-model` の既定の feature を切り、
    使う側 (ほかのランタイム層のクレート・ツール層・アプリ・適合テスト) に `features = ["std"]` (`ruzstd` は `["hash", "std"]`) を書いた。
    `glam` の既定の `all-types` はワークスペースの側で明示した。no_std に未対応のランタイム層のクレート (`ugoe-runtime` など) は、
    自分の feature `std` を持たないので、依存に直接 `std` を書いた (T0181〜T0183 で各クレートの `std` の feature に移す)。
  - 有効な feature が変わらないことは、すべてのメンバーごとに `cargo tree -e normal,build,dev -f '{p} {f}' --target all` を変更の前後で比べて
    確かめた。違いは次だけ: `glam` に `nostd-libm` が足され `libm` がビルドに入る (std があるときは std の数学関数を使うので結果は同じ)、
    `serde` に `alloc` が足される (`std` が有効なので中身は同じ)、`ugoe-core`・`ugoe-model` の feature `std`。`rmp-serde` は既定の feature を
    持たない (中の `rmp` の `std` を切れない) ので触っていない。no_std にするのは T0182。
  - `ugoe-core`・`ugoe-model` を `no_std` + `alloc` にし、既定で有効な feature `std` を足した。`std` を外しても公開 API は同じ
    (公開の型・関数に `HashMap` などは無かった。検証の中の `HashMap`・`HashSet` は `BTreeMap`・`BTreeSet` にした。キーは `Id` で `Ord` を持ち、
    走査の順で結果を作る箇所は無いので、結果は変わらない)。エラー型は thiserror 2 が `std` 無しでも `core::error::Error` を実装する。
  - `#![no_std]` は `#![cfg_attr(not(any(feature = "ts", feature = "schema")), no_std)]` にした。ts-rs と schemars の derive が、
    prelude の `String`・`format!`・`to_string` を前提にしたコードを生成し、`#![no_std]` ではコンパイルできないため。コード生成の feature は
    ツール層 (`ugoe-editor-core`) だけが有効にし、どちらも `std` を含む。ランタイムの構成 (既定・`std` 無し) では常に `no_std`。
  - フィクスチャ `smooth_arc` の `f32::sin_cos` (std にしかない) を `Vec2::from_angle` に替えた。std があるときの glam は `f32::sin_cos` を
    呼ぶので値はビット単位で同じ (`just gen` の公開仕様の例にも差分なし)。
  - `scripts/nostd-check.mjs` (対象は `NOSTD_CRATES`)・`just nostd-check`・CI の `rust (runtime)` のステップを足した。クレートは 1 つずつ
    `aarch64-unknown-none` で `cargo clippy --lib --no-default-features -- -D warnings` にかける (feature の統合で std が紛れ込まないように)。
    ターゲットが無ければ `rustup target add` の入れ方を出して止まる。`just groups` が一覧のクレートの存在と `std` の feature を検査する。
  - `bindings/godot/Cargo.lock` に `libm` を足した (godot のビルドは `--locked`)。
  - 範囲 (`crates:`) について: ワークスペースの依存の既定の feature を切ったので、ほかのクレートとアプリの `Cargo.toml` (依存の feature の行だけ)、
    `bindings/godot/Cargo.lock`、AGENTS.md・各 README・docs/testing.md も直した (受け入れ条件が求める変更)。
  - 検査: `just ci`・`just nostd-check`・`just wasm-check`・`just sdk-no-render-check`・`just cross-check`・`just publish-check`・`just licenses`・
    `just gen` (差分なし)・`cargo clippy`/`cargo test --manifest-path bindings/godot/Cargo.toml`・`node scripts/check-licenses.mjs --godot` が通った。
    手元に `aarch64-unknown-none` を `rustup target add` で入れた。
- やらなかったこと: 数学関数の `ugoe_core::math` と `glam` の `libm` (T0180)、ほかのランタイム層のクレートの no_std (T0181〜T0183)、
  `rust-toolchain.toml` の `targets` への `aarch64-unknown-none` の追加 (案内だけにした。CI はステップの中で入れる)。
- 起票の候補: なし (後続は T0180〜T0183 で起票済み)。T0180 では `ugoe-core` の `glam` の feature `nostd-libm` を `libm` に替える。
