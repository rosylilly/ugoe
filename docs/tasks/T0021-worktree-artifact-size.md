---
id: T0021
title: worktree ごとのビルドの成果物 (約 16GB) を減らす
status: todo
milestone: M9
crates: [Cargo.toml, justfile, scripts, docs]
depends_on: []
assignee:
---

# T0021: worktree ごとのビルドの成果物 (約 16GB) を減らす

## 目的

T0018 で mbx を入れて `target/` のコピーはやめたが、ワークスペースのクレートの dev のビルド (incremental を含む) は worktree ごとに作られ、
1 つあたり約 16GB を使う。並列のワーカーが 8 つあると 100GB を超える。dev のデバッグ情報を減らす
(`CARGO_PROFILE_DEV_DEBUG=0`・`debug = "line-tables-only"`・`split-debuginfo` など) と、どれだけ減り、何を失うか (デバッガ・バックトレース) を測って決める。

## 受け入れ条件

- [ ] 候補の設定ごとに、1 つの worktree の `target/` の大きさと `just ci` の時間を測り、表をメモに書いた
- [ ] 採る設定を決めた。開発の機械だけで使うもの (環境変数・`just wt-add` が書く設定) と、ワークスペースの `Cargo.toml` の `[profile.dev]` に書くものを分けて、理由を docs/workflow/build-cache.md に書いた
- [ ] パニックのバックトレースにファイルと行が出ることを確かめた (採る設定で)
- [ ] `just ci` が通る

## 参考

- T0018 のメモ (起票の候補)
- docs/workflow/build-cache.md・docs/workflow/multi-agent.md

## メモ

- T0019 で起票した (元の候補: T0018)。
