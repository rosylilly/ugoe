---
id: T0967
title: "mcp: Windows で --root の外を指す本体のエラーが \\\\?\\ の付いたパスを出し、テスト root_confines_module_bases が落ちる"
status: in-progress
milestone: M9
crates: [ugoe-mcp, docs]
depends_on: []
assignee: claude-worker-T0967
---

# T0967: mcp: Windows で --root の外を指す本体のエラーが `\\?\` の付いたパスを出し、テスト root_confines_module_bases が落ちる

## 目的

main の CI (https://github.com/rosylilly/ugoe/actions/runs/37456328831 、`rust (windows)` の Test) で次が落ちた
(T0965 の後。`the_mcp_bundle_relays_to_a_running_editor` は通った)。

```
thread 'root_confines_module_bases' panicked at crates\ugoe-mcp\tests\stdio.rs:1689:9:
rel.ugoeproj: base model \\?\C:\Users\runneradmin\AppData\Local\Temp\.tmpZ5cQ31\outside\dress-up.ugoe: outside the root directory `\\?\C:\Users\...\root` (...)
```

テストは `err.contains(outside_abs)` (`C:\...` の形) を確かめるが、サーバは canonicalize したパス (Windows では `\\?\C:\...` の verbatim) を
そのまま文に出している。利用者とエージェントに見せるパスとしても `\\?\` は読みにくく、エージェントがそのパスを次の呼び出しに使うと
`/` との混在で失敗しうる (T0965)。

## 受け入れ条件

- [ ] エラーや結果の文に出すパスで、`\\?\` (と `\\?\UNC\`) を通常の形に直して出すようにした (比べるのは canonicalize した形のまま。見せる所だけ直す)。ほかに同じくパスを見せる所 (ugoe-mcp の結果・エラー) を探し、そろえた
- [ ] 通常の形に直す関数を単体のテストで確かめた (Linux でも流れる文字列のテスト)。共有できるなら `ugoe-fs` に置くか検討し、置かないなら理由をメモに書く (`ugoe-fs` に置くならタスクの crates の外なのでメモに書く)
- [ ] `just cross-check` と `just ci` が通る。Windows の実機は無いので、次の main の CI の `rust (windows)` で確かめることをメモに書く

## 参考

- T0965・T0966 のメモ、crates/ugoe-mcp (--root の確かめ)、crates/ugoe-mcp/tests/stdio.rs

## メモ

- CI の失敗から起票した (2026-10-06)。
