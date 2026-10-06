---
id: T0959
title: "editor-core: Windows の clippy で、unix だけのテストが使う補助 (load_base_checked・save_module_project) が未使用になる"
status: todo
milestone: M9
crates: [ugoe-editor-core]
depends_on: []
assignee:
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

- [ ] import と補助を、使うテストと同じ `cfg` に揃える (`#[allow]` で黙らせない)
- [ ] `just cross-check` (Windows のターゲットの clippy) と `just crate ugoe-editor-core`・`just ci` が通る

## 参考

- T1607、docs/testing.md#別のターゲットの検査

## メモ

- CI の失敗から起票した (2026-10-06)。
