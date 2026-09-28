# CLAUDE.md

@AGENTS.md

## Claude Code 固有の補足

- 並行作業ではサブエージェントに `isolation: "worktree"` を使うか、`.worktrees/` 以下に worktree を作る。
- タスクの着手・完了には `.claude/skills/` の `task-start` / `task-finish` スキルを使う。
- 長時間のビルド (`just editor-build` など) はバックグラウンドで実行する。
