---
id: T0973
title: "mcp: Windows の CI で root_confines_module_bases が、一時ディレクトリの短い名前 (RUNNER~1) と正規化した名前の違いで落ちる"
status: in-progress
milestone: M9
crates: [ugoe-mcp]
depends_on: []
assignee: claude-worker-T0973
---

# T0973: mcp: Windows の CI で root_confines_module_bases が、一時ディレクトリの短い名前 (RUNNER~1) と正規化した名前の違いで落ちる

## 目的

T0967 の後の main の CI (https://github.com/rosylilly/ugoe/actions/runs/37460942079 、`rust (windows)`) で、エラーの文の `\\?\` は消えたが、
`root_confines_module_bases` (crates/ugoe-mcp/tests/stdio.rs) がまだ落ちた。

```
rel.ugoeproj: base model C:\Users\runneradmin\AppData\Local\Temp\.tmpczRQBa\outside\dress-up.ugoe: outside the root directory ...
```

テストは `err.contains(outside_abs)` で確かめるが、`outside_abs` は `tempfile::tempdir()` のパスそのもの。GitHub の Windows のランナーの
`TEMP` は 8.3 の短い名前 (`C:\Users\RUNNER~1\...`) なので、サーバが canonicalize して見せる `C:\Users\runneradmin\...` と文字列が合わない。
サーバの振る舞いは正しい (T0967 で見せるパスは通常の形になった)。テストの期待値を、サーバと同じく canonicalize して通常の形にしたものにする。

## 受け入れ条件

- [x] テストの期待値を `plain_path` (canonicalize した形) で作るようにした。ほかのテストで同じく canonicalize していない一時ディレクトリのパスを期待値にしている所が無いか探した
- [x] `just crate ugoe-mcp`・`just cross-check`・`just ci` が通る。次の main の CI の `rust (windows)` で確かめる

## 参考

- T0965・T0966・T0967

## メモ

- CI の失敗から起票した (2026-10-06)。
- 直したこと: `root_confines_module_bases` の期待値を `plain_path(outside_base.canonicalize())` にした。ほかの `tempfile::tempdir()` を使うテストは
  Windows の CI で通っており、パスを文字列で比べているのはこの 1 か所だけだった (`shown` を使う所は canonicalize したパスを渡している)。
- 確かめたこと: `just crate ugoe-mcp`、Windows のターゲットの clippy (`-p ugoe-mcp --all-targets`)、`just ci`。Windows の実機では次の main の CI で確かめる。
