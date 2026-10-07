---
id: T0186
title: "ugoe-format: png (std) の無い形の単体テストを CI で流す"
status: done
milestone: M11
crates: [ugoe-format, scripts, docs]
depends_on: []
assignee: claude-worker-T0186
---

# T0186: ugoe-format: png (std) の無い形の単体テストを CI で流す

## 目的

T0182 で `ugoe-format` を no_std にし、PNG と BCn の展開を feature `png` (`std` を含む) の側に置いた。ところが開発時の依存に
自分 (`ugoe-format = { path = ".", features = ["png"] }`) があるため、`cargo test -p ugoe-format --no-default-features --lib` でも
`png` と `std` が有効になり、`png` の無い形 (`ruzstd` の `std` が無い zstd の展開、`ReadLimits`、BCn の KTX2 を
`ImageError::UnsupportedKtx2` で断ること) の単体テストは流れない。T0182 では開発時の依存を一時的に外して一度だけ確かめた。
これを CI で流し続ける。

## 受け入れ条件

- [x] 開発時の依存の自分を外し、`png` を使う結合テスト (`tests/spec_bundles.rs` など) は `Cargo.toml` の `[[test]]` の
      `required-features = ["png"]` に、`png` を使う単体テストは `#[cfg(feature = "png")]` にした (どれを要るものにしたかをメモに書いた)
- [x] `cargo test -p ugoe-format --no-default-features --lib` で `png` と `std` が有効にならないことを確かめ
      (`cargo tree -p ugoe-format --no-default-features -e features` などで)、そのテストを `just nostd-check` (scripts/nostd-check.mjs。
      `ugoe-ffi` の std の無い単体テストと同じ所) か CI のグループから流すようにした
- [x] `png` を使うテストが `just ci` (ワークスペースのテスト) で今までどおり流れることを、前と後のテストの数で確かめた。`just crate ugoe-format` で
      `png` のテストが飛ばされるなら、その扱い (`--all-features` を足すなど) を決めてメモに書いた
- [x] crates/ugoe-format/README.md・docs/testing.md (と必要なら docs/portability.md) に、`png` の無い形のテストの流れ方を書いた
- [x] `just nostd-check`・`just wasm-check` と `just ci` が通る

## 参考

- T0182 のメモ (読み込みの上限・やらなかったこと・起票の候補)、crates/ugoe-format/Cargo.toml
- scripts/nostd-check.mjs (`NOSTD_CRATES`・`ugoe-ffi` の `--no-default-features` の単体テスト)、docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md

## メモ

- T0066 で起票した (元の候補: T0182 (`png` を外した `ugoe-format` の単体テストを CI で流す))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- 2026-10-07 (claude-worker-T0186) やったこと:
  - `crates/ugoe-format/Cargo.toml` の開発時の依存から自分 (`features = ["png"]`) を外した。`png` が要るものにしたのは、結合テストの
    `tests/broken_input.rs` (`decode_image_premultiplied`) と `tests/spec_bundles.rs` (`decode_png`) の 2 つ (`[[test]]` の
    `required-features = ["png"]`)。`compat.rs`・`spec_examples.rs`・`unity_fixtures.rs` は PNG をデコードしないのでそのまま。
    単体テストは T0182 で `#[cfg(feature = "png")]` が付いていたので足すものは無く、`png` 無しで出た `unused_mut` の警告
    (`image.rs` の `broken_ktx2_and_png_do_not_panic`) を `cfg_attr` で直しただけ。
  - `cargo tree -p ugoe-format --no-default-features -e features -i ugoe-format` で、前は `ugoe-format feature "png"`・`"std"`・`"default"`
    が開発時の依存から出ていたのが、後は出ない (`ruzstd` も `hash` だけ) ことを確かめた。
  - `scripts/nostd-check.mjs` に `HOST_TEST_CRATES` (`ugoe-format`・`ugoe-ffi`) を足し、`just nostd-check` がホストで
    `cargo clippy -p <c> --lib --profile test --no-default-features -- -D warnings` と `cargo test -p <c> --lib --no-default-features` を流す
    (`ugoe-ffi` の単体テストはここへ移した。clippy は新しく足した。`--tests` だと ugoe-ffi のテストでない形のライブラリをホストで作って
    失敗するので `--profile test`)。`nostd-check.mjs check` (`just groups`) が、`HOST_TEST_CRATES` が `NOSTD_CRATES` にあることと、
    開発時の依存に自分を置いていないことを検査する。CI のワークフローは変えていない (rust (runtime) の `just nostd-check` がそのまま流す)。
  - `png` の無い形の単体テストは 89 (`png` のある形は 110)。
  - テストの数: 前 (`cargo test -p ugoe-format`。開発時の依存で `png` が有効) は単体 110・結合 4+9+1+2+1。後の `just ci` (ワークスペース) と
    `just crate ugoe-format` も同じ 110・4+9+1+2+1。
  - `just crate ugoe-format` は、そのままだと `png` が無く結合テスト 2 つと単体テスト 21 が飛ばされるので、`just crate` を
    `scripts/ci-groups.mjs crate-args <crate>` (グループの `features` のうちそのクレートのもの) を付けて流すようにした。
    `--all-features` にしなかったのは、`gpu-tests` を持つクレートで GPU のテストまで流れるため。副作用として `just crate ugoe-model` も
    `schema`・`ts` を有効にして流す (CI の runtime グループと同じ。前は `schema` の `required-features` のテストが飛ばされていた)。
  - 文書: crates/ugoe-format/README.md (テストと feature)、docs/testing.md (rust (runtime) の行・`just crate` の feature・
    「std の無い形の単体テスト」の節)、docs/portability.md (`HOST_TEST_CRATES` への参照)。
  - 範囲 (`crates:`) の外: `justfile` の `crate` のレシピ (2 行とコメント)、`Cargo.lock` (自分への依存の 1 行が消えた)。
  - 検査: `just nostd-check`・`just wasm-check`・`just refs`・`just groups`・`just ci` が通った。
- やらなかったこと: crates/ugoe-ffi/README.md の `just nostd-check` の説明 (テストのコードの clippy を足したことは書いていない。範囲の外で、
  書いてあることは今も正しい)。`cargo test -p ugoe-format` を直接呼ぶと `png` のテストが飛ばされることは README と testing.md に書くだけにした。
- 起票の候補: 無し。
