---
id: T0079
title: "CI: Windows の just ffi-check が、C の見本のリンクで C ランタイム (動的と静的) が食い違って落ちる"
status: in-progress
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

- [ ] 原因を確かめ、メモに書いた (リンクの引数に CRT の指定が無い・既定のライブラリの順など)。出典 (clang・MSVC のリンカの文書) の URL を書く
- [ ] `scripts/ffi.mjs` の Windows のリンクの引数を直した (例: リンクにも `-fms-runtime-lib=dll` を渡す、`/NODEFAULTLIB:libucrt.lib` など)。手元の cargo-xwin と、WSL から本物の Windows の上で、見本の 3 つの実行が `ok` になることを確かめた
- [ ] push の後の CI の `rust (windows)` で確かめることをメモに書いた
- [ ] `just ffi-check` (Linux) と `just ci` が通る

## 参考

- T0069 のメモ、scripts/ffi.mjs、crates/ugoe-ffi/README.md

## メモ

- CI の失敗から起票した (2026-10-08)。
