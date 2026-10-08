---
id: T0079
title: "CI: Windows の just ffi-check が、C の見本のリンクで C ランタイム (動的と静的) が食い違って落ちる"
status: done
milestone: M11
crates: [scripts, .github/workflows, docs, crates/ugoe-ffi/README.md]
depends_on: []
assignee: claude-worker-T0079
---

# T0079: CI: Windows の just ffi-check が、C の見本のリンクで C ランタイム (動的と静的) が食い違って落ちる

## 目的

T0069 で Windows の CI (`rust (windows)` の C ABI の手順) でも `just ffi-check` を流すようにした。main の CI
(https://github.com/rosylilly/ugoe/actions/runs/37738790150 、`rust (windows)`) で、C の見本のリンクが次で落ちた。

```
LINK : warning LNK4098: defaultlib 'msvcrt.lib' conflicts with use of other libs; use /NODEFAULTLIB:library
LINK : warning LNK4217: symbol '__acrt_iob_func' defined in 'libucrt.lib(_file.obj)' is imported by 'sample-….o' in function 'main'
sample-….o : error LNK2019: unresolved external symbol __imp_fopen referenced in function read_file
(… __imp_perror・__imp_fseek・__imp_ftell・__imp_fread)
clang: error: linker command failed with exit code 1120
```

C の見本は `-fms-runtime-lib=dll` (動的の CRT の `__imp_*` を参照) でコンパイルしているのに、リンクでは静的の `libucrt.lib` が選ばれている。
コンパイルとリンクで CRT の指定がそろっていないか、Rust の静的ライブラリやランナーの既定のライブラリと食い違っている。手元の WSL からのクロス (cargo-xwin) では
通っていた (T0069 のメモ)。

## 受け入れ条件

- [x] 原因を確かめ、メモに書いた (リンクの引数に CRT の指定が無い・既定のライブラリの順など)。出典 (clang・MSVC のリンカの文書) の URL を書く
- [x] `scripts/ffi.mjs` の Windows のリンクの引数を直した (例: リンクにも `-fms-runtime-lib=dll` を渡す、`/NODEFAULTLIB:libucrt.lib` など)。手元の cargo-xwin と、WSL から本物の Windows の上で、見本の 3 つの実行が `ok` になることを確かめた
- [x] push の後の CI の `rust (windows)` で確かめることをメモに書いた
- [x] `just ffi-check` (Linux) と `just ci` が通る

## 参考

- T0069 のメモ、scripts/ffi.mjs、crates/ugoe-ffi/README.md

## メモ

- CI の失敗から起票した (2026-10-08)。

### 原因

- **clang の gcc 形式のドライバは、MSVC 向けのリンクの行に `-defaultlib:libcmt -defaultlib:oldnames` を必ず足す** (`-nostdlib` が無い限り)。
  `-fms-runtime-lib=dll` が効くのはコンパイルだけで、オブジェクトに `/DEFAULTLIB:msvcrt.lib /DEFAULTLIB:oldnames.lib` を埋める
  (`llvm-readobj --coff-directives` で確かめた) が、リンクの行の libcmt は消えない (`clang -### --target=x86_64-pc-windows-msvc
  -fms-runtime-lib=dll x.o` の出力で確かめた)。Rust の静的ライブラリも `/defaultlib:msvcrt` を求めるので、静的 (libcmt → libucrt) と
  動的 (msvcrt → ucrt) の両方が既定のライブラリに並ぶ。
- **MSVC の link.exe** はこの組み合わせで LNK4098 を出し、C の関数を静的な libucrt から解決するので、`__imp_fopen` などの動的の
  CRT の入口 (import library の ucrt.lib にしか無い) が未解決になる (LNK4217・LNK2019)。**lld-link は両方があっても msvcrt・ucrt から
  解決して通る**ので、T0069 の手元の確認 (cargo-xwin + clang + lld-link) では見えなかった。CI の clang は既定のリンカ (VS の link.exe) を使う。
- 出典: clang の `-fms-runtime-lib` (https://clang.llvm.org/docs/ClangCommandLineReference.html)、clang のドライバの MSVC のリンクの引数
  (https://github.com/llvm/llvm-project/blob/main/clang/lib/Driver/ToolChains/MSVC.cpp の `-defaultlib:libcmt`)、
  LNK4098 (https://learn.microsoft.com/en-us/cpp/error-messages/tool-errors/linker-tools-warning-lnk4098)、
  `/NODEFAULTLIB` (https://learn.microsoft.com/en-us/cpp/build/reference/nodefaultlib-ignore-libraries)、
  CRT の .lib の対応 (https://learn.microsoft.com/en-us/cpp/c-runtime-library/crt-library-features)。

### やったこと

- scripts/ffi.mjs: Windows の引数に `-Wl,-nodefaultlib:libcmt` を足した (clang がリンクの行に足す libcmt を外す。理由はコメントに書いた)。
  `/NODEFAULTLIB:` ではなく `-nodefaultlib:` と書いたのは、link.exe・lld-link のどちらも `-` の形を受け、`/` で始まる引数を
  Git for Windows の bash が経路に読み替える心配が無いため (ffi.mjs は bash を通さずに spawn するが、手で写すときのため)。
- docs/testing.md の「Windows と macOS」の C ABI の項に、C ライブラリを揃える方法と lld-link では見えない理由を足した。
  crates/ugoe-ffi/README.md は引数を書いていないので変えていない。
- **手元での再現と確認 (Linux の WSL2。Windows 11 の上):**
  - cargo-xwin で ugoe-ffi を x86_64-pc-windows-msvc の静的ライブラリにし、clang 18 で ffi.mjs と同じ引数で見本をコンパイル
    (Windows の VS 2022 (MSVC 14.44.35207) と Windows SDK 10.0.26100.0 のヘッダ)。**WSL の interop で本物の VS の link.exe を**、
    `clang -###` が出すリンクの行 (`-defaultlib:libcmt -defaultlib:oldnames ...`) そのままで動かすと、CI と同じ LNK4098・LNK4217・
    `__imp_fopen` などの 5 つの LNK2019 で落ちた (再現)。`-nodefaultlib:libcmt` を足すと警告も出ずにリンクでき、本物の Windows の上で
    見本の 3 つの実行 (avatar・`--lit` avatar-lit・dress-up と 4 つのモジュール) がすべて `ok` で終わった。
  - `clang -###` で `-Wl,-nodefaultlib:libcmt` がリンクの行に渡ることを確かめた。cargo-xwin の経路 (clang `-fuse-ld=lld` と xwin の
    CRT・SDK) でも、ffi.mjs の Windows の引数そのままでリンクでき (`-verbose` で libcmt を読まなくなった)、3 つが `ok`。
- Linux の `just ffi-check`・`just cross-check`・`just ci` が通った (Linux・macOS の引数は変えていない)。

### push の後の CI で確かめること (push していない)

- `rust (windows)` の「C ABI」で、リンクの LNK の警告とエラーが出ず、`header ok`・3 つの `ok`・
  `skip: the std-less static library is not checked on Windows` が出ること。
- 手元の clang は 18、windows-latest の LLVM はそれより新しい。新しい clang でもリンクの行の `-defaultlib:libcmt` は同じで、
  `-nodefaultlib:libcmt` は libcmt を足さなくなっていても害が無い。
- 落ちたら: link.exe の `-verbose` (`-Wl,-verbose`) で読んでいる CRT の .lib を見る。libucrt・libvcruntime が残るなら
  `-Wl,-nodefaultlib:libucrt -Wl,-nodefaultlib:libvcruntime` も足す。

### 起票の候補

- なし。
