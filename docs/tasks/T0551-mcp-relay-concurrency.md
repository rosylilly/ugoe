---
id: T0551
title: "MCP: --editor の中継で要求を並行に送り、取り消しを伝える"
status: todo
milestone: M9
crates: [ugoe-mcp]
depends_on: [T0529]
assignee:
---

# T0551: MCP: --editor の中継で要求を並行に送り、取り消しを伝える

## 目的

`ugoe-mcp --editor` の中継 (`src/relay.rs`) は標準入力を 1 行ずつ読み、エディタの答えを待ってから次の行を読む。
`apply_proposal`・`save_project` などはエディタの確認のダイアログを待つ (答えの時間は限らない) ので、その間は
同じクライアントのほかの要求 (`ping`・読むだけのツール) も、`notifications/cancelled` も届かない。MCP クライアントが
要求を並行に送る・時間切れで取り消す場合に、中継が止まって見える (T0529 のレビューで見つけた)。

stdio の MCP は要求を並行に送ってよく、応答の順も問わない。中継で要求ごとにエディタへの送信を並行にし、
`notifications/cancelled` を受けたら待っている確認を断れるようにする (エディタの側は `Approvals` で確認を拒否する口が要るか
を決める。HTTP の Streamable HTTP の取り消しの扱いも確かめる)。

## 受け入れ条件

- [ ] 中継が、確認を待つ要求の間もほかの要求に答える (テスト: 偽のエディタで確認を待たせ、同じ中継で `ping` と `list_proposals` が返る)
- [ ] 標準出力への書き込みは 1 行ずつ不可分 (並行の応答が混ざらない)
- [ ] `notifications/cancelled` の扱いを決め、README に書いた (伝えるなら、確認を待つ適用が拒否で終わることのテスト)
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-mcp/src/relay.rs`・`crates/ugoe-mcp/src/host.rs` (`Approvals`)
- ADR 0023、`docs/ai-integration.md#実行中のエディタにつなぐ-adr-0023`
- MCP の仕様の取り消し (`notifications/cancelled`)

## メモ

- T0529 のレビューで起票した。
