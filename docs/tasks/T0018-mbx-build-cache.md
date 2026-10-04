---
id: T0018
title: ビルドのキャッシュに mbx (mr-boxington) を使い、worktree の target/ のコピーをやめる
status: todo
milestone: M9
crates: [scripts, justfile, docs, AGENTS.md, CLAUDE.md, .claude]
depends_on: []
assignee:
---

# T0018: ビルドのキャッシュに mbx (mr-boxington) を使い、worktree の target/ のコピーをやめる

## 目的

`just wt-add` は main の `target/` (incremental を除いて約 30GB) を worktree ごとにコピーしている。
ext4 では reflink が効かないので 1 つ数分かかり、8 並列ではディスクを 250GB 以上使う。
main で `cargo` が `target/` を書き換えている最中にコピーすると失敗もする (M9 の並列の実行で起きた)。

[mbx (mr-boxington)](https://mr-boxington.jdx.dev/) は cargo と rustc を包むビルドのキャッシュで、
ワークスペース・target・レジストリ・ツールチェインのパスを置き換えたキー (portable keys) で成果物を 1 つの置き場に持つ。
同じリポジトリの worktree どうしで成果物を共有できるので、`target/` のコピーが要らなくなる (利用者の提案。2026-10-04)。

## 受け入れ条件

- [ ] 開発の機械で mbx を使う手順を docs に書いた (インストール、`mbx setup`、`mbx doctor` での確認、無効にする `MBX_DISABLE=1`、片付けの `mbx gc`・`mbx clean`)
- [ ] `scripts/worktree.mjs` の `add` は、mbx が有効なら `target/` をコピーせず、無効なら今までどおりコピーする (判定のしかたを doc コメントに書く)
- [ ] `docs/workflow/multi-agent.md`・`AGENTS.md`・`CLAUDE.md`・`.claude/skills/task-start` の「`target/` のコピー」の記述を直した
- [ ] 注意点を docs に書いた
  - reflink の無いファイルシステム (ext4 など) では、戻した成果物がハードリンクで読み取り専用になる。mbx を通さずに cargo を動かすと `output file ... is not writeable` で失敗しうるので、`mbx clean` か `cargo clean` で直す
  - cargo の incremental のコンパイルは共有の置き場を通らない (ワークスペースのクレートの dev のビルドは各 worktree で作られる)
- [ ] mbx が有効な状態で、新しい worktree の `just crate ugoe-runtime` と `just ci` が通り、依存クレートが置き場から戻ることを `mbx` の統計で確かめた (かかった時間をメモに書く)
- [ ] CI (GitHub Actions) に入れるかを決めた (入れないなら理由を書く)
- [ ] `just ci` が通る

## 参考

- https://mr-boxington.jdx.dev/installation、https://mr-boxington.jdx.dev/how-it-works
- `scripts/worktree.mjs`、docs/workflow/multi-agent.md

## メモ

- この機械への導入 (`cargo install mbx --locked`・`mbx setup`) は、利用者の了承を得て、並列のワーカーが止まっている間にオーケストレーターが行う (途中で wrap を入れると、読み取り専用の成果物で mbx を通らない cargo が失敗しうるため)。
