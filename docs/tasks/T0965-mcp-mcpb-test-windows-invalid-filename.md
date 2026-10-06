---
id: T0965
title: "mcp: Windows の CI で、展開した .mcpb の manifest のコマンドを起動するテストが InvalidFilename で落ちる"
status: todo
milestone: M9
crates: [ugoe-mcp, docs]
depends_on: []
assignee:
---

# T0965: mcp: Windows の CI で、展開した .mcpb の manifest のコマンドを起動するテストが InvalidFilename で落ちる

## 目的

main の CI (https://github.com/rosylilly/ugoe/actions/runs/37447416182 、`rust (windows)` の Test = `just test-os`) で次が落ちた。

```
thread 'the_mcp_bundle_relays_to_a_running_editor' panicked at crates\ugoe-mcp\tests\editor.rs:266:14:
called `Result::unwrap()` on an `Err` value: Os { code: 123, kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect." }
```

266 行目は `Relay::spawn_command` の `spawn()`。テストは manifest の `server.mcp_config.command` の `${__dirname}` を展開先のディレクトリで置き換えて起動している。
`/` 区切りのコマンドと Windows のパス (`\\?\` の付いた形など) が混ざって、Windows が受け付けない名前になっている疑い。
テストの誤りか、manifest (`ugoe-mcp --mcpb` が書く `entry_point`・`command`) が Windows の Claude Desktop でも同じく起動できない誤りかを確かめる。

前の run (37429758787) では Windows の clippy が先に落ちていたので、このテストは Windows で流れていなかった。

## 受け入れ条件

- [ ] 原因を確かめ、メモに書いた (テストの置き換え方か、manifest の書き方か)
- [ ] manifest の誤りなら manifest を直す (MCPB の仕様の `${__dirname}` と Windows の区切り。出典の URL を書く)。テストの誤りならテストを直す
- [ ] Windows の振る舞いを手元で確かめられない場合は、置き換えたパスの形を単体のテストで確かめ、確かめていないことをメモに書く
- [ ] `just cross-check` と `just ci` が通る

## 参考

- crates/ugoe-mcp (mcpb)、crates/ugoe-mcp/tests/editor.rs、docs/ai-integration.md (MCP の配布)

## メモ

- CI の失敗から起票した (2026-10-06)。
