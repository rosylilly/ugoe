---
id: T0069
title: "CI: Windows で just ffi-check の std がある形 (C の見本のリンクと実行) を流す"
status: todo
milestone: M11
crates: [scripts, .github/workflows, docs, crates/ugoe-ffi/README.md]
depends_on: []
assignee:
---

# T0069: CI: Windows で just ffi-check の std がある形 (C の見本のリンクと実行) を流す

## 目的

T0067 で、CI の rust (macos) が `just ffi-check` を流すようになった。Windows では流さないと決めた (scripts/ffi.mjs のリンクは
gcc 形式の引数の `cc` と Linux・macOS のシステムのライブラリ (`system`) を前提にしていて、windows-latest には `cc` が無く、MSVC は
開発者のシェル (vcvars) の外では使えないため)。今、Windows の `ugoe-ffi` は unity.yml の `native (windows-x86_64)` がビルドするだけで、
C の見本 (`examples/c/sample.c`) を静的ライブラリにリンクして動かしてはいない。scripts/ffi.mjs に Windows のリンクを足し、
rust (windows) で std がある形の C の見本が動くことを見続ける。std の無い形は今までどおり飛ばす (MSVC の C ライブラリに `aligned_alloc` が無い)。

## 受け入れ条件

- [ ] scripts/ffi.mjs が Windows で C の見本をコンパイル・リンクできる (MSVC の `cl` か `clang-cl`・`clang` から選び、理由をコメントに書く。
      リンクするシステムのライブラリは `cargo rustc -p ugoe-ffi --lib --crate-type staticlib -- --print native-static-libs` の出力に合わせる。
      `-Werror -pedantic` に当たる警告の扱いも決める)。Linux・macOS の引数は変えない
- [ ] `.github/workflows/ci.yml` の rust (windows) で `just ffi-check` を流す (MSVC を使うなら `ilammy/msvc-dev-cmd` などで開発者の環境を作る。
      cbindgen の入れ方と `!cancelled()` の扱いは T0067 の macOS の手順に合わせる)
- [ ] 手元で Windows を使えなければ、使えないことと、push した後の CI で確かめること (見本の 3 つの実行が通ること・時間の延び) をメモに書いた
- [ ] docs/testing.md#windows-と-macos と crates/ugoe-ffi/README.md の、どの OS で C の見本を確かめているかの記述を直した (T0067 で書いた「Windows では流さない」の理由も)
- [ ] `just ci` が通る

## 参考

- T0067 のメモ (Windows で流さないと決めた理由・起票の候補)、scripts/ffi.mjs (`windows`・`system`)、justfile の `ffi-check`
- docs/testing.md#windows-と-macos、.github/workflows/unity.yml (`native (windows-x86_64)`)

## メモ

- T0068 で起票した (元の候補: T0067 (Windows で `just ffi-check` の std がある形を流す))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- ci.yml は T0072 (rust-checks) も触るが、別のジョブなので依存は付けていない。
