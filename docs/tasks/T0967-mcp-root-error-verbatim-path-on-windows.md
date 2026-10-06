---
id: T0967
title: "mcp: Windows で --root の外を指す本体のエラーが \\\\?\\ の付いたパスを出し、テスト root_confines_module_bases が落ちる"
status: done
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

- [x] エラーや結果の文に出すパスで、`\\?\` (と `\\?\UNC\`) を通常の形に直して出すようにした (比べるのは canonicalize した形のまま。見せる所だけ直す)。ほかに同じくパスを見せる所 (ugoe-mcp の結果・エラー) を探し、そろえた
- [x] 通常の形に直す関数を単体のテストで確かめた (Linux でも流れる文字列のテスト)。共有できるなら `ugoe-fs` に置くか検討し、置かないなら理由をメモに書く (`ugoe-fs` に置くならタスクの crates の外なのでメモに書く)
- [x] `just cross-check` と `just ci` が通る。Windows の実機は無いので、次の main の CI の `rust (windows)` で確かめることをメモに書く

## 参考

- T0965・T0966 のメモ、crates/ugoe-mcp (--root の確かめ)、crates/ugoe-mcp/tests/stdio.rs

## メモ

- CI の失敗から起票した (2026-10-06)。
- やったこと (2026-10-06):
  - `crates/ugoe-mcp/src/verbatim.rs` を足した。`plain_path` (パス 1 つ)・`plain_paths_in` (文の途中のパスもすべて)・
    JSON のすべての文字列にかける `plain_paths_in_value`。直すのは `\\?\<ドライブ>:\` と `\\?\UNC\` だけ (`\\?\C:` の
    ように `\` が続かないもの・`\\?\Volume{…}` などは通常の形で書けないのでそのまま)。文字列だけを見るので Linux でも
    単体のテストが流れる。
  - 見せる所を 1 か所にまとめた: ツールの結果 (`tools.rs` の `tool_result`) で、成功の `structuredContent` とその文、
    添える文、エラーの文・`structuredContent` (`args`・`cause`・`items` を含む) の文字列をすべて直す。落ちたテストの
    `base model <パス>` は editor-core の `ProjectError::BaseRefused` が作る文なので、個々の `display()` を直すだけでは
    届かない。結果の境で直せば editor-core 由来の文・エディタに組み込んだときの結果も同じになる。ほかに、
    `initialize` の `instructions` (エディタのプロジェクトのパス) と、起動時のログ (`paths confined to …`) も直した。
    root の判定・プロジェクトの鍵・読み書きは正規化した形のまま (エージェントが通常の形を渡しても、`resolve` が
    正規化して同じ鍵になる)。
  - `wsl.rs` の `to_wsl` の `\\?\` 外しを `plain_path` に置き換えた。テストの `desktop_dirname` (tests/editor.rs) も
    `plain_path` を使う。
  - Windows で結果のパスを `canonicalize` の形と比べていたテストを、通常の形と比べるようにした
    (tests/stdio.rs の `shown`、`apps/editor/src-tauri/src/mcp_tests.rs` の 2 か所)。**`apps/editor` はタスクの crates の外**
    だが、テストの期待を合わせるだけの小さな変更なので行った (直さないと Windows の CI で落ちる)。
  - README の「パスを限る」に、見せるパスは通常の形にすることを書いた。
- `ugoe-fs` に置かなかった理由: 使うのは今は ugoe-mcp (とそのテスト・エディタの MCP のテスト) だけで、ugoe-fs は
  読み書きの補助 (I/O) のクレート。見せる形の変換は MCP の結果の約束 (エージェントに見せるパス) に属する。
  ほかのアプリやクレートで要るようになったら移す。
- 確かめたこと: `just crate ugoe-mcp`・`just crate ugoe-editor`・`just cross-check`・`just ci` が通る (Linux)。
  **Windows の実機では確かめていない。** `root_confines_module_bases` などが通るかは、次の main の CI の
  `rust (windows)` で確かめる。
- 起票の候補: なし。

