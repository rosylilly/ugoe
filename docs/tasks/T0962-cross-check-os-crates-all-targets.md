---
id: T0962
title: "just cross-check: Windows のターゲットの clippy に、OS ごとのクレートのテストのコード (--all-targets) を加える"
status: done
milestone: M9
crates: [scripts, docs]
depends_on: []
assignee: claude-worker-T0962
---

# T0962: just cross-check: Windows のターゲットの clippy に、OS ごとのクレートのテストのコード (--all-targets) を加える

## 目的

CI の `rust (windows)` は `ugoe-format`・`ugoe-fs`・`ugoe-editor-core`・`ugoe-cli`・`ugoe-mcp` を `--all-targets` で clippy にかけるが、
手元の `just cross-check` (`scripts/cross-check.mjs`) は Windows のターゲットで `ugoe-ai`・`ugoe-mcp` の `--all-targets` しか見ない。
そのため T0959 の失敗 (unix だけのテストが使う補助が Windows で未使用) を手元で見つけられなかった (T0959 の候補)。

## 受け入れ条件

- [x] `scripts/cross-check.mjs` の Windows (と、必要なら macOS) のターゲットの clippy に、CI の `rust (windows)` の Clippy と同じクレートの `--all-targets` を加えた (一覧は 1 か所で持ち、CI と食い違わないようにする。CI 側の一覧と共有できるなら共有する)
- [x] T0959 の修正を戻すと `just cross-check` が失敗することを確かめた (確かめた後に戻す)
- [x] docs/testing.md#別のターゲットの検査 を直した
- [x] `just cross-check` と `just ci` が通る

## 参考

- T0959、scripts/cross-check.mjs、.github/workflows の rust (windows)、justfile の lint-os

## メモ

- T0959 のワーカーの報告から起票した (2026-10-06)。
- やったこと: `scripts/cross-check.mjs` の `tools` (x86_64-pc-windows-gnu・--all-features・--all-targets) のクレートを
  `ugoe-ai` と `scripts/ci-groups.mjs` の `OS_CRATES` (CI の rust (windows) の `just lint-os` の一覧) にした。一覧は `OS_CRATES` を
  import して 1 か所で持つ。テスト (scripts/cross-check.test.mjs) は OS_CRATES がすべて入り重複が無いことを確かめる。
  docs/testing.md#別のターゲットの検査 の表と説明、#windows-と-macos の OS_CRATES の手順を直した。
- T0959 の修正 (96f5d8ac の crates/ の差分) を一時的に戻すと `node scripts/cross-check.mjs run tools` が
  `unused import: load_base_checked`・`function save_module_project is never used` で失敗することを確かめ、戻した。
- macOS のターゲットは足さなかった: ring の C に Apple の SDK が要り ubuntu では検査できない。rust (macos) が macOS の上で
  `just lint-os` を流すので足りる。
- `just cross-check` (vcam・apps・tools) と `just ci` が通った (Linux の手元)。
