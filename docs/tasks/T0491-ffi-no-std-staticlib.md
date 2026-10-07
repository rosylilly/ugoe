---
id: T0491
title: ugoe-ffi を std の無い環境向けの静的ライブラリとしてビルドする
status: done
milestone: M11
crates: [ugoe-ffi, Cargo.toml, Cargo.lock, justfile, scripts, .github/workflows, docs]
depends_on: [T0183, T0184, T0489]
assignee: claude-worker-T0491
---

# T0491: ugoe-ffi を std の無い環境向けの静的ライブラリとしてビルドする

## 目的

ゲーム機や組み込み機器のように Rust の `std` が無い環境に、C ABI を静的ライブラリとして組み込めるようにする
([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md#std-の無い環境への-c-abi))。

## 受け入れ条件

- [x] `ugoe-ffi` が既定で有効な feature `std` を持ち、外すと `#![no_std]` でビルドできる
- [x] `std` を外した形のアロケータ (`#[global_allocator]`) は、ホストの C ライブラリの `malloc`・`free` (と整列の要る確保の関数) を呼ぶ。
      パニックの処理 (`#[panic_handler]`) は `abort` を呼ぶ。どちらも SDK ごとの知識を持たない
- [x] `std` の有無で関数の集合とヘッダが変わらない (エラーは T0489 の `UgoeError` で返すので、スレッドローカルは要らない)。
      `std` を外した形でパニックの処理が `abort` になることを README に書いた
- [x] `std` を外した形で使えない関数 (テクスチャのデコードなど) をヘッダと README に書いた
- [x] `aarch64-unknown-none` で staticlib をビルドし、C の見本とリンクして、未解決のシンボルがホストの C ライブラリのもの (`malloc`・`free`・
      `abort` など) だけであることを確かめる検査を足し、CI で回す (実機での確認は対象外。メモに書く)
- [x] docs/portability.md のターゲット別の表と C ABI の規約、crates/ugoe-ffi/README.md に、std の無い形のビルドと組み込みの手順を書いた
- [x] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、crates/ugoe-ffi/README.md、docs/portability.md#c-abi-ugoe-ffi-の規約

## メモ

- 2026-10-06 起票。`panic=abort` は Cargo のプロファイルで決まるので、ワークスペースのプロファイルを変えずに済む方法 (専用のプロファイル) を選ぶ。
- T0180・T0183 からの申し送り (2026-10-07): ugoe-ffi を no_std にするとき、ugoe-ffi と ugoe-format にも clippy.toml (std の数学関数の disallowed-methods) を足し、scripts/nostd-check.mjs の MATH_CRATES に入れる。`ugoe-ffi/src/lighting.rs` の `f32::sqrt` は `ugoe_core::math::sqrt` にする。
- T0184 からの申し送り (2026-10-07): T0184 がランタイム層の 7 つのクレートで有効にした clippy の lint (`expect_used`・`indexing_slicing`・`panic`・`string_slice`・`unimplemented`・`unreachable`) を ugoe-ffi にも有効にする (std 無しでは C ABI の中のパニックがアプリ全体を止めるため)。docs/portability.md の「パニックしない」を見る。
- 2026-10-07 実装 (claude-worker-T0491)。
  - **やったこと:**
    - `ugoe-ffi` に既定で有効な feature `std` (`serde_json/std`・`ugoe/std`・`ugoe-format/png`) を足した。クレートは
      `#![cfg_attr(not(test), no_std)]` で `core`・`alloc` から書き、`catch_unwind` を使う `std` の形だけ `extern crate std;`。
    - `src/nostd.rs` (`#[cfg(all(not(feature = "std"), not(test)))]`): アロケータは `malloc`・`calloc`・`realloc`・`free` と、
      `malloc` の整列 (64 ビットで 16、ほかで 8) を超える整列の `aligned_alloc` (C11) を呼ぶ。パニックの処理は `abort`。
      `calloc`・`realloc` は ADR に無かったが、LLVM が確保と 0 埋めを `calloc` に置き換えるのでシンボルはどのみち要る。明示して使った
      (ADR 0031 に追記)。`aligned_alloc` を `posix_memalign` より選んだ理由も追記に書いた。
    - ルートの `Cargo.toml` に `[profile.nostd]` (release を継ぎ `panic = "abort"`)。dev・release は変えていない。
    - `std` の無い形の `ugoe_image_decode` は PNG と BCn の KTX2 を `UGOE_ERROR_FORMAT`・`"unsupported_texture"` で断る
      (関数とヘッダは同じ)。ヘッダ (関数の説明と `UGOE_ERROR_PANIC`)・README に書いた。単体テストを足した。
    - 申し送り: `clippy.toml` (数学関数の禁止) を `ugoe-ffi` と `ugoe-format` に足し、`MATH_CRATES` に入れた。`lighting.rs` の
      `f32::sqrt` と結合テストの `sin`・`cos` を `ugoe::core::math` に。T0184 の 6 つの lint を `ugoe-ffi` で有効にし、添字を
      `get` と `error::at` (範囲外は `UGOE_ERROR_OUT_OF_RANGE`) に直した。`BLEND_SHAPE_NAMES` (コンパイル時の表) だけ `#[expect]`。
    - 検査: `just nostd-check` (`NOSTD_CRATES` に `ugoe-ffi`) が clippy に加えて、ホストで `std` の無い形の単体テストと、
      `aarch64-unknown-none` の staticlib をフリースタンディングの C の見本 (`examples/c/nostd.c`) と clang + rust-lld でリンクし、
      未解決のシンボルが `abort`・`aligned_alloc`・`calloc`・`free`・`malloc`・`realloc` だけか、それらを形だけ定義すればリンクが通るかを確かめる。
      `just ffi-check` はホスト向けの `std` の無い staticlib を C ライブラリとリンクし (`examples/c/nostd_main.c`)、見本の 3 つのモデルで実行する
      (Windows は MSVC に `aligned_alloc` が無いので飛ばす)。手元で ASan を付けても漏れ・不正なアクセスは無かった (検査には入れていない)。
    - 文書: crates/ugoe-ffi/README.md#std-の無い環境、docs/portability.md (ターゲット別の表・C ABI の規約・NOSTD_CRATES・パニックしない)、
      docs/testing.md、ADR 0031 の追記、justfile と ci.yml のコメント。
  - **範囲外の変更:** `crates/ugoe-format/clippy.toml` (申し送りどおり。中身は他のクレートと同じで、lint 以外は変えていない)。
  - **確かめたこと:** `just nostd-check`・`just ffi-check`・`just refs`・`just conformance` (すべてのバインディング)・`just unity-check`・
    `just cross-check`・`just licenses`・`just wasm-check`・`just ci` が手元 (Linux x86_64) で通った。
  - **確かめていないこと:** ゲーム機の実機と SDK、`aarch64-unknown-none` で作ったものを動かすこと (リンクまで)。macOS での `just ffi-check` の
    std の無い形 (`aligned_alloc` は macOS 10.15 以降にある)。
  - **CI で確かめること (push していない):** ci.yml はコメントだけ変えた。rust (runtime) の `just nostd-check` は C の見本のコンパイルに
    `clang` を使う (ubuntu-latest に入っている想定。無ければ `apt-get install clang` の手順を足す)。rust (bindings) の `just ffi-check` が
    `--profile nostd` のビルドを 1 回増やす (手元で 20 秒ほど)。
  - **注意:** `std` を外した `ugoe-ffi` はホスト (Linux など) で cdylib・結合テストとしてはビルドできない (`core` が巻き戻しありで配られているため。
    staticlib を `--profile nostd` で作り、`rust_eh_personality` を形だけ定義すれば動く)。README に書いた。
  - **起票の候補:**
    - macOS の CI で `just ffi-check` (std の無い形を含む) を流す。
    - パニックの処理でメッセージをホストに渡す口 (ホストが登録するログの関数)。今は `abort` だけ。
    - `-Zbuild-std` が使えるターゲット (tier 3 のゲーム機のターゲットなど) でのビルドの手順の確認。
