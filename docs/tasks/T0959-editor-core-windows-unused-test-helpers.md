---
id: T0959
title: "editor-core: Windows の clippy で、unix だけのテストが使う補助 (load_base_checked・save_module_project) が未使用になる"
status: done
milestone: M9
crates: [ugoe-editor-core]
depends_on: []
assignee: claude-worker-T0959
---

# T0959: editor-core: Windows の clippy で、unix だけのテストが使う補助 (load_base_checked・save_module_project) が未使用になる

## 目的

main の CI (https://github.com/rosylilly/ugoe/actions/runs/37429758787 、`rust (windows)` の Clippy) が次で失敗する。

```
error: unused import: `load_base_checked`
  --> crates\ugoe-editor-core\src\read_limits_tests.rs:10:22
error: function `save_module_project` is never used
  --> crates\ugoe-editor-core\src\read_limits_tests.rs:41:4
```

`read_limits_tests.rs` (T1607) で、これらを使うテストが `#[cfg(unix)]` なので、Windows では使われない。

## 受け入れ条件

- [x] import と補助を、使うテストと同じ `cfg` に揃える (`#[allow]` で黙らせない)
- [x] `just cross-check` (Windows のターゲットの clippy) と `just crate ugoe-editor-core`・`just ci` が通る

## 参考

- T1607、docs/testing.md#別のターゲットの検査

## メモ

- CI の失敗から起票した (2026-10-06)。
- やったこと: `read_limits_tests.rs` の `load_base_checked` の import と `save_module_project` に `#[cfg(unix)]` を付け、使うテスト (`fifos_in_a_module_project_and_its_base_are_refused`) と揃えた。
- 確かめたこと: 修正の前に `cargo clippy -p ugoe-editor-core --all-features --all-targets --target x86_64-pc-windows-gnu -- -D warnings` で CI と同じ 2 つのエラーを再現し、修正の後は通ることを確かめた (`aarch64-apple-darwin` も通る)。`just cross-check`・`just crate ugoe-editor-core`・`just ci` も通る。実機の Windows では流していない。
- 起票の候補: `just cross-check` (scripts/cross-check.mjs) は Windows のターゲットで `ugoe-editor-core` などの OS_CRATES の `--all-targets` を検査しない (tools は ugoe-ai・ugoe-mcp だけ) ので、今回の失敗は `just cross-check` では見つからない。OS_CRATES (ugoe-format・ugoe-fs・ugoe-editor-core・ugoe-cli) を Windows のターゲットの clippy に加えると、CI の rust (windows) の前に手元で見つけられる。
