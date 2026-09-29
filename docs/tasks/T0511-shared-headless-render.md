---
id: T0511
title: ヘッドレス描画を CLI と MCP サーバで共有する
status: in-progress
milestone: M6
crates: [ugoe-cli, ugoe-mcp, scripts, Cargo.toml, commitlint.config.mjs, docs]
depends_on: [T0508, T0114]
assignee: claude-worker-T0511
---

# T0511: ヘッドレス描画を CLI と MCP サーバで共有する

## 目的

`ugoe render` の中身 (ヘッドレスの GPU の用意・出力の大きさと `fit`・PNG の書き出し・時間を進める処理) を
ライブラリとして切り出し、`ugoe-mcp` の `render_png` が子プロセスではなく同じ関数を呼ぶようにする。

## 受け入れ条件

- [ ] 共有の置き場所 (ugoe-cli をライブラリにするか、新しいツール層のクレートか) を決めて書く
- [ ] `ugoe render` の出力が変わらない (既存の GPU テストが通る)
- [ ] `ugoe-mcp` が `ugoe` のバイナリなしで PNG を描ける。GPU のドライバの不調でサーバが落ちない手当てを残す
- [ ] `just ci`・`just layers`・`just licenses` が通る

## 参考

- `docs/tasks/T0508-mcp-server.md` のメモ
- `crates/ugoe-mcp/README.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
