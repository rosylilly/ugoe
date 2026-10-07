---
id: T0067
title: "CI: macOS で just ffi-check (std の無い静的ライブラリの形を含む) を流す"
status: todo
milestone: M11
crates: [.github/workflows, scripts, docs]
depends_on: []
assignee:
---

# T0067: CI: macOS で just ffi-check (std の無い静的ライブラリの形を含む) を流す

## 目的

T0491 で `just ffi-check` (scripts/ffi.mjs) が、ホスト向けの `std` の無い `ugoe-ffi` の静的ライブラリ (`--profile nostd`) を
C ライブラリとリンクして見本のモデルで動かすようになった。CI では rust (bindings) のグループ (ubuntu) だけで流れていて、macOS では
確かめていない (`aligned_alloc` は macOS 10.15 以降にある。リンクのフラグは `-framework CoreFoundation`)。macOS の CI の
rust (macos) のジョブで `just ffi-check` を流し、C ABI の見本 (std がある形と無い形) が macOS でもリンクでき、動くことを見続ける。

## 受け入れ条件

- [ ] `.github/workflows/ci.yml` の `rust-os` の macOS で `just ffi-check` を流す手順を足した (cbindgen の入れ方は rust-group の
      「Install cbindgen」に合わせる。前の手順が落ちても流す `!cancelled()` の扱いもほかの手順と揃える)。Windows で流すか
      (std がある形だけ。std の無い形は MSVC に `aligned_alloc` が無いので飛ばす) も決め、理由をコメントに書いた
- [ ] 手元で macOS を使えなければ、使えないことと、push した後の CI で確かめることをメモに書いた (macOS の実機・CI で通ったなら、その結果を書いた)
- [ ] docs/testing.md (CI のジョブ・別の OS の検査) と crates/ugoe-ffi/README.md#std-の無い環境 の、どの OS で確かめているかの記述を直した
- [ ] `just ci` が通る

## 参考

- T0491 のメモ (確かめていないこと・起票の候補)、scripts/ffi.mjs (`checkNostd`)、justfile の `ffi-check`
- docs/testing.md#windows-と-macos、docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md

## メモ

- T0066 で起票した (元の候補: T0491 (macOS の CI で std の無い形を含む `just ffi-check` を流す))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
