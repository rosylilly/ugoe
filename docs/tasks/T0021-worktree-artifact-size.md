---
id: T0021
title: worktree ごとのビルドの成果物 (約 16GB) を減らす
status: done
milestone: M9
crates: [Cargo.toml, justfile, scripts, docs]
depends_on: []
assignee: claude-worker-T0021
---

# T0021: worktree ごとのビルドの成果物 (約 16GB) を減らす

## 目的

T0018 で mbx を入れて `target/` のコピーはやめたが、ワークスペースのクレートの dev のビルド (incremental を含む) は worktree ごとに作られ、
1 つあたり約 16GB を使う。並列のワーカーが 8 つあると 100GB を超える。dev のデバッグ情報を減らす
(`CARGO_PROFILE_DEV_DEBUG=0`・`debug = "line-tables-only"`・`split-debuginfo` など) と、どれだけ減り、何を失うか (デバッガ・バックトレース) を測って決める。

## 受け入れ条件

- [x] 候補の設定ごとに、1 つの worktree の `target/` の大きさと `just ci` の時間を測り、表をメモに書いた
- [x] 採る設定を決めた。開発の機械だけで使うもの (環境変数・`just wt-add` が書く設定) と、ワークスペースの `Cargo.toml` の `[profile.dev]` に書くものを分けて、理由を docs/workflow/build-cache.md に書いた
- [x] パニックのバックトレースにファイルと行が出ることを確かめた (採る設定で)
- [x] `just ci` が通る

## 参考

- T0018 のメモ (起票の候補)
- docs/workflow/build-cache.md・docs/workflow/multi-agent.md

## メモ

- T0019 で起票した (元の候補: T0018)。
- 実装 (2026-10-04、claude-worker-T0021):
  - 採る設定: **.worktrees/ の下の worktree だけ** `debug = "line-tables-only"`。`just wt-add` (scripts/worktree.mjs) が `.worktrees/.cargo/config.toml` を書く (内容と判定は scripts/lib/worktree-cargo-config.mjs、テストは scripts/worktree-cargo-config.test.mjs を `just script-check` に足した)。cargo は実行したディレクトリから上へ設定を探すので、just を通らない cargo (lefthook の pre-push・rust-analyzer) にも効き、main には効かない。1 行目のしるしが無い (手で直した) ファイルは書き直さない。
  - **ルートの Cargo.toml の `[profile.dev]` は変えていない** (main と、worktree を使わない開発者のデバッグの体験はそのまま)。just のレシピに環境変数を足す案は、pre-push などと設定が食い違って両方の成果物が溜まるので採らなかった。理由は docs/workflow/build-cache.md#worktree-のデバッグ情報、wt-add の説明は multi-agent.md に足した。
  - 計測の表は docs/workflow/build-cache.md#計測-t0021 に書いた。要点 (1 つの worktree、`mbx clean` の後の `just ci`、target 全体 / 固有 (nlink=1) / 時間): `debug = true` 21.0GiB / 11.9〜14.3GiB / 275〜324 秒、`line-tables-only` 9.8GiB / 2.7GiB (2 回目。置き場に無い 1 回目は 8.0GiB) / 138 秒 (2 回目。1 回目は 459 秒)、`split-debuginfo = "unpacked"` 11.8GiB / 10.2GiB / 372 秒 (mbx が 121 件 bypassed)、`debug = 0` 4.9GiB / 3.9GiB / 359 秒 (バックトレースにファイルと行が出ない)。
  - **計測の条件:** ほかに 7 つ前後のワーカーが並行してビルドしていた (load average 7〜33、24 コア)。時間は ±数十% の幅で見る。`debug = true` の 2 回目は間に置き場の gc があり 96 misses だった。計測した時点の main では apps/editor の ToolOverlay.test.ts が 1 件落ちて web-check で止まった (rebase 後の main では通る) ので、計測の時間は fmt-check〜web-check の分。
  - バックトレース: `line-tables-only` で、ワークスペースの一時的なテスト (ugoe-core の tests/ に置いて消した) のパニックが `at ./tests/tmp_t0021_bt.rs:4:18` と出ることを確かめた。親のディレクトリの `.cargo/config.toml` が `-C debuginfo=line-tables-only` として効くことも、一時ディレクトリの入れ子の構成で確かめた。`debug = 0` では関数の名前だけになる。
  - 最後の `just ci` (rebase 後、`CARGO_PROFILE_DEV_DEBUG=line-tables-only` を付けて。設定ファイルと同じ効き目): 通った。326 秒、target 10.2GiB (固有 5.7GiB。main が進んでいたので misses あり)。
- **マージ後の注意 (オーケストレーター向け):** `.worktrees/.cargo/config.toml` は .worktrees/ の下の **既存の worktree にもすぐ効く**。書かれた直後の各 worktree の次のビルドは依存クレートも含めて作り直し (1 回目は数分長い)、古い設定の成果物も残る。ワーカーが止まっている間に `just wt-add` を流すか、手で書いた後に既存の worktree で `mbx clean` するとよい。このタスクの中では実際の `.worktrees/.cargo/config.toml` は書いていない (並行するワーカーのビルドを崩さないため)。
- mbx が無効な機械では `just wt-add` は設定を書かない (コピーする main の target/ は `debug = true` の成果物で、設定が違うと使われないため)。すでに書かれていれば効くので、その機械では消すかを利用者が決める。
- やらなかったこと: macOS・Windows では確かめていない (`split-debuginfo` の既定が違う。macOS は `unpacked` が既定)。
- 起票の候補:
  - ルートの `[profile.dev]` を `line-tables-only` にするか (全員のデバッガの体験が変わるので、利用者の判断が要る)。

- **判断 (2026-10-05、利用者の判断):** デバッグ情報を line-tables-only にするのは worktree だけ。ルートの [profile.dev] は変えない。
