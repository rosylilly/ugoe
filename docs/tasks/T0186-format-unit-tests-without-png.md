---
id: T0186
title: "ugoe-format: png (std) の無い形の単体テストを CI で流す"
status: todo
milestone: M11
crates: [ugoe-format, scripts, docs]
depends_on: []
assignee:
---

# T0186: ugoe-format: png (std) の無い形の単体テストを CI で流す

## 目的

T0182 で `ugoe-format` を no_std にし、PNG と BCn の展開を feature `png` (`std` を含む) の側に置いた。ところが開発時の依存に
自分 (`ugoe-format = { path = ".", features = ["png"] }`) があるため、`cargo test -p ugoe-format --no-default-features --lib` でも
`png` と `std` が有効になり、`png` の無い形 (`ruzstd` の `std` が無い zstd の展開、`ReadLimits`、BCn の KTX2 を
`ImageError::UnsupportedKtx2` で断ること) の単体テストは流れない。T0182 では開発時の依存を一時的に外して一度だけ確かめた。
これを CI で流し続ける。

## 受け入れ条件

- [ ] 開発時の依存の自分を外し、`png` を使う結合テスト (`tests/spec_bundles.rs` など) は `Cargo.toml` の `[[test]]` の
      `required-features = ["png"]` に、`png` を使う単体テストは `#[cfg(feature = "png")]` にした (どれを要るものにしたかをメモに書いた)
- [ ] `cargo test -p ugoe-format --no-default-features --lib` で `png` と `std` が有効にならないことを確かめ
      (`cargo tree -p ugoe-format --no-default-features -e features` などで)、そのテストを `just nostd-check` (scripts/nostd-check.mjs。
      `ugoe-ffi` の std の無い単体テストと同じ所) か CI のグループから流すようにした
- [ ] `png` を使うテストが `just ci` (ワークスペースのテスト) で今までどおり流れることを、前と後のテストの数で確かめた。`just crate ugoe-format` で
      `png` のテストが飛ばされるなら、その扱い (`--all-features` を足すなど) を決めてメモに書いた
- [ ] crates/ugoe-format/README.md・docs/testing.md (と必要なら docs/portability.md) に、`png` の無い形のテストの流れ方を書いた
- [ ] `just nostd-check`・`just wasm-check` と `just ci` が通る

## 参考

- T0182 のメモ (読み込みの上限・やらなかったこと・起票の候補)、crates/ugoe-format/Cargo.toml
- scripts/nostd-check.mjs (`NOSTD_CRATES`・`ugoe-ffi` の `--no-default-features` の単体テスト)、docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md

## メモ

- T0066 で起票した (元の候補: T0182 (`png` を外した `ugoe-format` の単体テストを CI で流す))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
