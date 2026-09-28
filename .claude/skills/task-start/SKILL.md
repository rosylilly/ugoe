---
name: task-start
description: docs/tasks/ のタスクに着手する (担当の宣言、worktree とブランチの作成、関連文書の読み込み)。「T0104 をやって」「次のタスクに着手」などで使う。
---

# タスクに着手する

手順の正は `docs/workflow/multi-agent.md`。引数としてタスク ID (例: `T0104`) を受け取る。ID がなければ `just tasks --ready` の結果から選び、ユーザーに確認する。

1. `docs/tasks/<ID>-*.md` を読み、次を確認する。
   - `status` が `todo` である (`in-progress` なら、他のエージェントが担当しているので止まって報告する)
   - `depends_on` のタスクがすべて `done` である (`just tasks` で確認する)
2. main の作業ツリーで、タスクファイルの `status: in-progress` と `assignee:` (自分のセッション名など) を更新し、そのファイルだけをコミットする (コミットメッセージは英語)。
   `git commit -m "chore(tasks): start <ID>" docs/tasks/<ID>-*.md`
3. 自分がすでに隔離された worktree の中にいる場合 (サブエージェントを `isolation: "worktree"` で起動した場合など) は、ブランチ `task/<ID>-<slug>` を作るだけにする。そうでなければ次を実行する。
   `git worktree add .worktrees/<ID> -b task/<ID>-<slug>` を実行し、以降はそのディレクトリで作業する。
   worktree の中で `pnpm install --offline` を実行する (Git フックが worktree 内の node_modules を使うため)。
4. 次の文書を読む: `AGENTS.md`、タスクファイルの「参考」、`crates:` に書かれた各クレートの `README.md`。
5. 受け入れ条件を TODO リストに書き写してから、作業を始める。
