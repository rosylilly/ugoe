---
id: T0513
title: MCP: 自動リギングの判断を外部のエージェントに任せるツール
status: in-progress
milestone: M6
crates: [ugoe-mcp, ugoe-ai, docs]
depends_on: [T0508, T0503, T0504]
assignee: claude-worker-T0513
---

# T0513: MCP: 自動リギングの判断を外部のエージェントに任せるツール

## 目的

ADR 0022 の「外部のエージェントに判断を任せる経路」を作る。ugoe は API を呼ばず、MCP クライアント側のエージェント (Claude Code・OpenCode・Codex CLI など) が、画像を見て分類し、パラメータを選ぶ。

## 受け入れ条件

- [ ] PSD からプロジェクトを作る (または開く) → レイヤーの一覧と画像 (合成画像・レイヤーの縮小画像。MCP の image コンテンツ) を返す → エージェントが分類の結果 (T0503 の語彙) を渡す → 提案 (T0504 の `propose_rig` など) を作って検証の結果と差分を返す、までのツール
- [ ] エージェントが `RigParams` など判断の値を渡せる。検証に通らなければ、エラーをツールの結果として返し、エージェントが直して呼び直せる
- [ ] 分類の語彙・出力の形は `ugoe_ai` の JSON Schema から作り、手で写さない
- [ ] 提案の適用は既存の `apply_proposal` と同じく、利用者の確認 (MCP クライアントの許可) を通す (ADR 0006)
- [ ] stdio でサーバを起動し、JSON-RPC を流すテストで一通りの流れを確かめる。Claude Code・OpenCode から使う手順を `crates/ugoe-mcp/README.md` に書く
- [ ] `just ci` が通る

## 参考

- `docs/adr/0022-multiple-ai-providers-and-agents.md`
- `docs/tasks/T0508-mcp-server.md`・`docs/tasks/T0504-auto-rig.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
