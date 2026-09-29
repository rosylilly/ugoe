---
id: T0514
title: MCP: HTTP のトランスポート
status: todo
milestone: M6
crates: [ugoe-mcp, docs]
depends_on: [T0513]
assignee:
---

# T0514: MCP: HTTP のトランスポート

## 目的

ChatGPT のように HTTP で MCP サーバにつなぐクライアントのために、`ugoe-mcp` に stdio に加えて HTTP (MCP の Streamable HTTP) のトランスポートを足す (ADR 0022)。

## 受け入れ条件

- [ ] `ugoe-mcp --http <addr>` で HTTP のトランスポートで待ち受ける。既定はループバックだけに束ねる
- [ ] 認証 (トークンなど) と、外部に公開するときの注意 (トンネルを使う場合など) を README に書く。認証なしで外部に公開できないようにする
- [ ] 新しい外部の依存を入れるなら、ライセンス (`just licenses`) と重さを確かめ、理由を書く
- [ ] HTTP で一通りのツールを呼ぶテスト。ChatGPT から使う手順を書く (実際の接続の確認は利用者に残してよい)
- [ ] `just ci` が通る

## 参考

- `docs/adr/0022-multiple-ai-providers-and-agents.md`
- https://modelcontextprotocol.io/

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
