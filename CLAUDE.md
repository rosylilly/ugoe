# CLAUDE.md

@AGENTS.md

## Claude Code 固有の補足

- 並行作業では、オーケストレーターが `just wt-add <ID>` で worktree を作り、そのパスをサブエージェントに渡す。`isolation: "worktree"` は `target/` のコピーも mbx の判定 ([docs/workflow/build-cache.md](docs/workflow/build-cache.md)) も行わないので使わない。
- Bash のシェルはセッションを始めたときの PATH のままなので、cargo は just のレシピ (`just crate` など) から呼ぶ。直接呼ぶなら `export PATH="$HOME/.local/share/mbx/bin:$PATH";` を先頭に付ける (mbx を入れている機械で)。
- タスクの着手・完了には `.claude/skills/` の `task-start` / `task-finish` スキルを使う。
- 長時間のビルド (`just editor-build` など) はバックグラウンドで実行する。
