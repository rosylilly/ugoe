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
3. 自分がすでにこのタスクの worktree の中にいる場合 (オーケストレーターが用意した場合) は、そのまま作業する。
   そうでなければ main の作業ツリーで `just wt-add <ID>` を実行し、以降は `.worktrees/<ID>` で作業する。
   (worktree とブランチの作成、`pnpm install --offline` まで行われる。mbx が無効なら main の target/ もコピーする。
   `git worktree add` を直接使うと、mbx が無効な機械では依存クレートを最初からビルドし直すことになるので使わない。
   docs/workflow/build-cache.md)
4. 次の文書を読む: `AGENTS.md`、タスクファイルの「参考」、`crates:` に書かれた各クレートの `README.md`。
5. 受け入れ条件を TODO リストに書き写してから、作業を始める。
