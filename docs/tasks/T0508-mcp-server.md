---
id: T0508
title: MCP サーバ (`ugoe-mcp`)
status: in-progress
milestone: M6
crates: [ugoe-mcp, scripts, Cargo.toml, commitlint.config.mjs, docs]
depends_on: [T0601]
assignee: claude-worker-T0508
---

# T0508: MCP サーバ (`ugoe-mcp`)

## 目的

`Command` の JSON Schema をツールとして公開する MCP サーバを作り、Claude Code などの外部のエージェントがプロジェクトを読み、編集を提案・適用できるようにする。

## 受け入れ条件

- [ ] 新しいクレート `ugoe-mcp` (ツール層、MIT)。`scripts/check-layers.mjs` と commitlint の scope に登録する
- [ ] ツール: プロジェクトを開く・保存する、モデルの要約を読む (パーツ・メッシュ・デフォーマ・パラメータ・キーの格子)、`Command` を適用する (検証と ApplyReport を返す)、Undo、指定したパラメータの値で PNG を描く (`ugoe render` と同じ)
- [ ] ツールの入力の型は `Command` の JSON Schema から作る (手で写さない)。AI は提案するだけという原則 (ADR 0006) との関係を決めて書く (例: 適用には確認を求める設定)
- [ ] 実行中のエディタにつなぐか、`.ugoeproj` を直接扱うかを決めて書く
- [ ] Claude Code からつないで、簡単な編集 (パラメータの作成とキーの追加) ができることを確かめ、手順を書く
- [ ] `just ci` が通る

## 参考

- `docs/ai-integration.md` の原則 5
- https://modelcontextprotocol.io/

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
