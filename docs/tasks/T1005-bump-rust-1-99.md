---
id: T1005
title: "Rust のツールチェインを 1.99.0 に上げる"
status: in-progress
milestone: Release
crates: [rust-toolchain.toml, crates, apps, bindings, fuzz, tests, scripts, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T1005
---

# T1005: Rust を 1.99.0 に上げる

## 目的

`just tool-updates` (T1004) が、固定している Rust (1.98.1) より新しい 1.99.0 を知らせた。利用者の決定 (2026-10-12): 上げる。

## 受け入れ条件

- [ ] `rust-toolchain.toml` と、T1004 の `PINNED_TOOLS` が挙げる写し (ワークフロー・文書など) を 1.99.0 にした (`just tool-updates --list` の写しの検査が通る)
- [ ] 新しい clippy の警告・rustfmt の差・非推奨を直した (`-D warnings`)。`#[allow]` で黙らせたものは理由を書いた
- [ ] `just ci`・`just wasm-check`・`just nostd-check`・`just cross-check`・`just vcam-check`・`just conformance`・`just godot-check`・`just fuzz-check` (nightly があれば)・`just perf` が通る。perf の値を前と比べてメモに書いた
- [ ] mbx のキャッシュと CI の rust-cache の鍵 (T1000) が新しい版で一度外れることを docs に書く必要があれば書いた

## 参考

- T1004 のメモ (`just tool-updates`)、T1000 (rust-cache の鍵)、rust-toolchain.toml、docs/release.md#固定した道具の版

## メモ

- 利用者の決定 (2026-10-12) を受けてオーケストレーターが起票した。
