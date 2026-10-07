---
id: T0069
title: "CI: Windows で just ffi-check の std がある形 (C の見本のリンクと実行) を流す"
status: done
milestone: M11
crates: [scripts, .github/workflows, docs, crates/ugoe-ffi/README.md]
depends_on: []
assignee: claude-worker-T0069
---

# T0069: CI: Windows で just ffi-check の std がある形 (C の見本のリンクと実行) を流す

## 目的

T0067 で、CI の rust (macos) が `just ffi-check` を流すようになった。Windows では流さないと決めた (scripts/ffi.mjs のリンクは
gcc 形式の引数の `cc` と Linux・macOS のシステムのライブラリ (`system`) を前提にしていて、windows-latest には `cc` が無く、MSVC は
開発者のシェル (vcvars) の外では使えないため)。今、Windows の `ugoe-ffi` は unity.yml の `native (windows-x86_64)` がビルドするだけで、
C の見本 (`examples/c/sample.c`) を静的ライブラリにリンクして動かしてはいない。scripts/ffi.mjs に Windows のリンクを足し、
rust (windows) で std がある形の C の見本が動くことを見続ける。std の無い形は今までどおり飛ばす (MSVC の C ライブラリに `aligned_alloc` が無い)。

## 受け入れ条件

- [x] scripts/ffi.mjs が Windows で C の見本をコンパイル・リンクできる (MSVC の `cl` か `clang-cl`・`clang` から選び、理由をコメントに書く。
      リンクするシステムのライブラリは `cargo rustc -p ugoe-ffi --lib --crate-type staticlib -- --print native-static-libs` の出力に合わせる。
      `-Werror -pedantic` に当たる警告の扱いも決める)。Linux・macOS の引数は変えない
- [x] `.github/workflows/ci.yml` の rust (windows) で `just ffi-check` を流す (MSVC を使うなら `ilammy/msvc-dev-cmd` などで開発者の環境を作る。
      cbindgen の入れ方と `!cancelled()` の扱いは T0067 の macOS の手順に合わせる)
- [x] 手元で Windows を使えなければ、使えないことと、push した後の CI で確かめること (見本の 3 つの実行が通ること・時間の延び) をメモに書いた
- [x] docs/testing.md#windows-と-macos と crates/ugoe-ffi/README.md の、どの OS で C の見本を確かめているかの記述を直した (T0067 で書いた「Windows では流さない」の理由も)
- [x] `just ci` が通る

## 参考

- T0067 のメモ (Windows で流さないと決めた理由・起票の候補)、scripts/ffi.mjs (`windows`・`system`)、justfile の `ffi-check`
- docs/testing.md#windows-と-macos、.github/workflows/unity.yml (`native (windows-x86_64)`)

## メモ

- T0068 で起票した (元の候補: T0067 (Windows で `just ffi-check` の std がある形を流す))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- ci.yml は T0072 (rust-checks) も触るが、別のジョブなので依存は付けていない。

### 止まった理由 (利用者の判断が要る)

- **`.github/workflows/ci.yml` の編集が、作業環境の権限 (auto mode の分類器の「共有の資源の変更」) で拒まれた。** 受け入れ条件 2 だけが
  残っている。迂回はしていない。オーケストレーターか利用者が、この worktree で次の変更を入れてコミットすれば完了する
  (`rust-os` の C ABI の 2 つの手順から `&& matrix.name == 'macos'` を外し、コメントを Windows に合わせる。cbindgen の入れ方と
  `!cancelled()` は T0067 の macOS の手順のまま):

  ```yaml
      # C ABI (`just ffi-check`): rust (bindings) と同じく、ヘッダが最新かと、C の見本を静的ライブラリにリンクして実行する。
      # macOS は std がある形と std の無い形の両方で、macOS の C ライブラリ (`aligned_alloc` は macOS 10.15 以降) とリンクのフラグ
      # (`-framework CoreFoundation`) を見続ける (T0067。crates/ugoe-ffi/README.md#std-の無い環境)。Windows は std がある形だけで、
      # ランナーに入っている clang (Visual Studio と Windows SDK を自分で探すので vcvars は要らない) で MSVC のライブラリとリンクする
      # (T0069。std の無い形は MSVC の C ライブラリに `aligned_alloc` が無いので scripts/ffi.mjs が飛ばす)。
      # cbindgen は rust (bindings) の「Install cbindgen」と同じ版を入れる。
      - name: Install cbindgen
        if: ${{ !cancelled() }}
        uses: taiki-e/install-action@v2
        with:
          tool: cbindgen@0.29.4
      - name: C ABI
        if: ${{ !cancelled() }}
        run: just ffi-check
  ```

- docs/testing.md と crates/ugoe-ffi/README.md は、この変更が入った後の形 (rust (windows) でも流す) で書いてある。ci.yml を変えずに
  マージするなら、その 2 か所を戻す必要がある。

### やったこと

- scripts/ffi.mjs: Windows では C の見本を `clang` (gcc 形式の引数のドライバ。`CC` で変えられる) でコンパイル・リンクする。
  - **`cl` ではなく `clang` にした理由** (コメントにも書いた): clang は Visual Studio と Windows SDK を自分で探す (VS の Setup の COM の
    API とレジストリ) ので、開発者のシェル (vcvars) を作る手順 (`ilammy/msvc-dev-cmd` などの外部の action) が要らない。
    警告の扱い `-std=c11 -Wall -Wextra -Werror -pedantic` を Linux・macOS と同じ引数のまま使える (`cl` なら `/W4 /WX` に読み替え、
    警告の番号ごとの差を別に確かめる必要がある)。GitHub の windows-latest のイメージには LLVM (clang) が入っている。
  - **システムのライブラリ:** `cargo xwin rustc --target x86_64-pc-windows-msvc -p ugoe-ffi --lib --crate-type staticlib -- --print native-static-libs`
    の出力が `kernel32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib /defaultlib:msvcrt` (Rust 1.98.1)。これを
    `-lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp` と、C ライブラリを msvcrt (動的) に揃える `-fms-runtime-lib=dll` にした。
    clang の既定は静的な libcmt で、lld-link の `/verbose` で libcmt を読んでいることを確かめた (link.exe なら LNK4098 の警告になる組み合わせ)。
    出典: rustc の `--print native-static-libs` (https://doc.rust-lang.org/rustc/command-line-arguments.html#--print-print-compiler-information)、
    clang の `-fms-runtime-lib` (https://clang.llvm.org/docs/ClangCommandLineReference.html)。
  - **`-Werror -pedantic` の扱い:** そのまま使う。MSVC の C ライブラリは標準の `fopen` に非推奨の印 (`_CRT_INSECURE_DEPRECATE`) を付けていて
    `-Wdeprecated-declarations` で止まった (手元で再現) ので、`-D_CRT_SECURE_NO_WARNINGS` で消す (標準の C の関数を使い続けるため)。
    MSVC のヘッダはシステムのヘッダとして扱われ、`-pedantic` の警告は出なかった。
  - std の無い形は今までどおり飛ばす (`skip: ...` を出す)。Linux・macOS の引数は変えていない (手元の Linux で `just ffi-check` が通る)。
- **手元での確認 (Linux の WSL2。Windows 11 (10.0.26200) の上):**
  - cargo-xwin で ugoe-ffi を x86_64-pc-windows-msvc の静的ライブラリにし、clang 18 (`--target=x86_64-pc-windows-msvc -fuse-ld=lld`、
    xwin の MSVC の CRT と Windows SDK のヘッダ・ライブラリ) で ffi.mjs と同じ引数でコンパイル・リンクし、**WSL の interop で本物の
    Windows の上で** 3 つの実行 (avatar・`--lit` avatar-lit・dress-up と 4 つのモジュール) がすべて `ok` で終わった。Wine でも同じ。
  - ffi.mjs の Windows の分岐そのものも、`process.platform` を `win32` に偽る `--import` の前処理 (cargo を cargo-xwin に、exe の引数を
    Windows のパスに替える。リポジトリには入れていない) で流し、`header ok`・3 つの `ok`・`skip: the std-less static library ...` が出た。
- 文書: docs/testing.md (CI のジョブの表の rust (windows)・rust (macos)、「Windows と macOS」の C ABI の項 (T0067 の「Windows では
  流さない」の理由を、clang を選んだ理由に置き換えた)、「CI のキャッシュ」の rust (windows) の増分)、crates/ugoe-ffi/README.md
  (ヘッダの生成の C コンパイラと CI のジョブ、「範囲外」の「Windows と macOS での C の見本のビルドは確かめていない」を消した)。
- `just ci` が通った (main に rebase した後)。

### 確かめていないこと (CI で確かめる。push していない)

- windows-latest (windows-2025) の `clang` が PATH にあり、既定のターゲットが `x86_64-pc-windows-msvc` で、Visual Studio と Windows SDK を
  自分で見つけて `link.exe` でリンクできること (手元の Windows には VS 2022 はあるが clang が無く、clang は Linux から xwin と lld-link で
  代わりに確かめた)。ステップの shell は bash (Git for Windows) なので、PATH の `/usr/bin/link.exe` (coreutils) ではなく VS の
  `link.exe` を clang が選ぶこと。
  - 落ちたら: clang が無ければ `C:\Program Files\LLVM\bin` を PATH に足すか `CC` で指す。リンカの取り違えなら `CC="clang -fuse-ld=lld"`
    ではなく ffi.mjs の Windows の引数に `-fuse-ld=lld` を足す (LLVM に lld-link が入っている)。未解決のシンボルなら `--print native-static-libs`
    を windows-latest で流し、ffi.mjs の `system.win32` を合わせる。
- `taiki-e/install-action` が Windows に cbindgen 0.29.4 を入れられること (ビルド済みが無ければ cargo-binstall か cargo install に落ちる。時間が延びる)。
- 「C ABI」で `header ok` と 3 つの `ok`、`skip: the std-less static library is not checked on Windows` が出ること。
- rust (windows) の時間の延び (見込み: `cargo build -p ugoe-ffi` で ugoe-* と 10 個ほどの依存。キャッシュが当たれば 1 分ほど) と、
  main の 2 回目の run でキャッシュ (`v0-rust-windows-…`) が完全一致で戻ること。

### 起票の候補

- なし (ci.yml の 1 か所が入れば完了する)。
- オーケストレーターが利用者の承認を得て ci.yml の rust-os の 2 つの手順の `if:` から `matrix.name == 'macos'` を外し、コメントを直した (2026-10-07。ワーカーの権限では ci.yml の編集が拒否されたため)。push の後の CI で確かめることは上のとおり。
