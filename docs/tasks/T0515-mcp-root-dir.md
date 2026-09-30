---
id: T0515
title: MCP: ツールが触れるパスを 1 つのディレクトリの下に限る
status: in-progress
milestone: M6
crates: [ugoe-mcp, docs]
depends_on: [T0514]
assignee: claude-worker-T0515
---

# T0515: MCP: ツールが触れるパスを 1 つのディレクトリの下に限る

## 目的

`ugoe-mcp` を HTTP (T0514) で外に出すと、`open_project`・`save_project`・`import_psd` などが利用者の権限で任意のパスを読み書きできる。
ツールが触れるパスを、起動時に指定した 1 つのディレクトリの下に限る (T0514 のメモ)。

## 受け入れ条件

- [ ] `--root <dir>` で、ツールが読み書きするパスを `<dir>` の下に限る (正規化してから判定し、`..` やシンボリックリンクで外に出られない)
- [ ] HTTP で待ち受けるときの既定の扱いを決めて書く (例: `--root` か `--read-only` のどちらかを必須にする)。stdio の既定は変えない
- [ ] 外に出るパスを拒否するテスト (相対パス・`..`・シンボリックリンク)
- [ ] README と `docs/ai-integration.md` を更新し、`just ci` が通る

## 参考

- `docs/tasks/T0514-mcp-http-transport.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
