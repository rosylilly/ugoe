---
id: T0974
title: "mcp: T0973 の後、macOS の CI で root_confines_module_bases が落ちる (絶対パスで書いた本体は書いたままの形で見せる)"
status: in-progress
milestone: M9
crates: [ugoe-mcp]
depends_on: []
assignee: claude-worker-T0974
---

# T0974: mcp: T0973 の後、macOS の CI で root_confines_module_bases が落ちる (絶対パスで書いた本体は書いたままの形で見せる)

## 目的

T0973 で `root_confines_module_bases` の期待値を、3 つのプロジェクトすべて canonicalize した形にした。main の CI
(https://github.com/rosylilly/ugoe/actions/runs/37464594287 、`rust (macos)`) で次が落ちた。

```
abs.ugoeproj: base model /var/folders/.../outside/dress-up.ugoe: outside the root directory `/private/var/folders/.../root` (...)
```

サーバは、相対の `module.base` は canonicalize した root から解決した形で、絶対の `module.base` は書いたままの形で見せる。macOS の一時ディレクトリは
`/var` (`/private/var` への symlink) なので、絶対パスの `abs.ugoeproj` では canonicalize した形と合わない。T0973 の前はこの絶対パスの場合だけで
比べ方が合っていた。

## 受け入れ条件

- [ ] 相対の参照 (`rel`・`dots`) は canonicalize した通常の形、絶対の参照 (`abs`) は書いたままの形 (`outside_abs`) を期待値にした
- [ ] `just crate ugoe-mcp`・`just ci` が通る。次の main の CI の `rust (macos)`・`rust (windows)` で確かめる

## 参考

- T0973

## メモ

- CI の失敗から起票した (2026-10-06)。オーケストレーターの T0973 の直し方の誤り。
