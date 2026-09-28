# CLAUDE.md

@AGENTS.md

## Claude Code 固有の補足

- 並行作業では、オーケストレーターが `just wt-add <ID>` で worktree を作り、そのパスをサブエージェントに渡す。`isolation: "worktree"` は `target/` をコピーしないためビルドが遅くなるので使わない。
- タスクの着手・完了には `.claude/skills/` の `task-start` / `task-finish` スキルを使う。
- 長時間のビルド (`just editor-build` など) はバックグラウンドで実行する。
