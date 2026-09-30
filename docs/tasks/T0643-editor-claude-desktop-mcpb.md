---
id: T0643
title: エディタ: Claude Desktop 用の MCP Bundle (.mcpb)
status: in-progress
milestone: M6
crates: [apps/editor, docs]
depends_on: [T0642]
assignee: claude-worker-T0643
---

# T0643: エディタ: Claude Desktop 用の MCP Bundle (.mcpb)

## 目的

Claude Desktop から、実行中のエディタにつなげるようにする (ADR 0023 の続き。利用者の要望)。
`ugoe-mcp --editor` の中継を MCP Bundle (`.mcpb`) に包み、エディタの「Connect agents」から Claude Desktop に入れられるようにする。

## 受け入れ条件

- [ ] MCPB の最新の仕様 (manifest の版・`server` の `type: binary`・`entry_point`・`mcp_config`・`compatibility` の OS・`tools` の宣言・`user_config` など) を公式の文書で確かめ、`ugoe-mcp --editor` を包む `manifest.json` を作る。ツールの一覧は `ugoe-mcp` の `tools/list` から作り、手で写さない
- [ ] `.mcpb` の作り方: 配布用のビルドで OS ごとの `.mcpb` を作ってエディタに同梱する。検証 (公式の `mcpb` の CLI の validate など) を通す。開発モードでは `target/` の `ugoe-mcp` から作れる
- [ ] エディタの「Connect agents」に Claude Desktop の手順: `.mcpb` を開いて Claude Desktop に入れるボタン (または保存)、入れた後の確かめ方。エディタが Windows で動くときは Windows 版の `ugoe-mcp.exe` を包む
- [ ] 操作マニュアル (`docs/manual/mcp.md`) に Claude Desktop の節
- [ ] `.mcpb` の中身 (manifest の検証と、中の `ugoe-mcp --editor` が起動して偽のエディタにつながること) を確かめるテスト
- [ ] `just ci`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0023-editor-mcp-bridge.md`
- `docs/tasks/T0642-editor-mcp-bridge.md`・`docs/tasks/T0516-mcp-document-host.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
