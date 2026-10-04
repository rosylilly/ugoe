---
id: T0018
title: ビルドのキャッシュに mbx (mr-boxington) を使い、worktree の target/ のコピーをやめる
status: done
milestone: M9
crates: [scripts, justfile, docs, AGENTS.md, CLAUDE.md, .claude]
depends_on: []
assignee: claude-worker-T0018
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

- [x] 開発の機械で mbx を使う手順を docs に書いた (インストール、`mbx setup`、`mbx doctor` での確認、無効にする `MBX_DISABLE=1`、片付けの `mbx gc`・`mbx clean`)
- [x] `scripts/worktree.mjs` の `add` は、mbx が有効なら `target/` をコピーせず、無効なら今までどおりコピーする (判定のしかたを doc コメントに書く)
- [x] `docs/workflow/multi-agent.md`・`AGENTS.md`・`CLAUDE.md`・`.claude/skills/task-start` の「`target/` のコピー」の記述を直した
- [x] 注意点を docs に書いた
  - reflink の無いファイルシステム (ext4 など) では、戻した成果物がハードリンクで読み取り専用になる。mbx を通さずに cargo を動かすと `output file ... is not writeable` で失敗しうるので、`mbx clean` か `cargo clean` で直す
  - cargo の incremental のコンパイルは共有の置き場を通らない (ワークスペースのクレートの dev のビルドは各 worktree で作られる)
- [x] mbx が有効な状態で、新しい worktree の `just crate ugoe-runtime` と `just ci` が通り、依存クレートが置き場から戻ることを `mbx` の統計で確かめた (かかった時間をメモに書く)
- [x] CI (GitHub Actions) に入れるかを決めた (入れないなら理由を書く)
- [x] `just ci` が通る

## 参考

- https://mr-boxington.jdx.dev/installation、https://mr-boxington.jdx.dev/how-it-works
- `scripts/worktree.mjs`、docs/workflow/multi-agent.md

## メモ

- この機械への導入 (`cargo install mbx --locked`・`mbx setup`) は、利用者の了承を得て、並列のワーカーが止まっている間にオーケストレーターが行う (途中で wrap を入れると、読み取り専用の成果物で mbx を通らない cargo が失敗しうるため)。
- 実装 (2026-10-04、claude-worker-T0018):
  - justfile: mbx の shim (`data_local_directory()/mbx/bin/cargo`) があれば PATH の先頭に足す (Unix のみ)。起動ファイルを直していないシェルからでも、レシピの cargo は mbx を通る (最後の `just ci` はシェルの cargo が `/snap/bin/cargo` のまま mbx を通った)。`script-check` のレシピ (scripts/mbx.test.mjs) を足し、`just ci` に入れた。
  - `scripts/lib/mbx.mjs`: 有効の判定 (`MBX_DISABLE` が無いか 0・false、かつ PATH の最初の cargo の隣に `mbx-target` がある)。`scripts/worktree.mjs add` は有効ならコピーしない。main の `target/` がシンボリックリンク (managed target) のときも、リンクを写して main と同じ target を指さないようコピーしない。`remove` は worktree の `target` が managed target なら `mbx clean <path>` も行う (T9998 で確かめた: 24.3GiB を片付けた)。
  - 手順と注意点は docs/workflow/build-cache.md (新規。docs/README.md の表にも足した)。multi-agent.md・AGENTS.md・CLAUDE.md・task-start の記述を直した。
  - 開発者に mbx を求めるかは **任意** (無ければ従来どおりコピー) にした。並行して worktree を多く作る機械では推奨と書いた。
- **見つかった問題と回避 (範囲外の小さな変更):** mbx 1.22.0 はビルドスクリプトを `#!/bin/sh` のラッパーで包むので、dash では名前に `:` を含む `DEP_*` の環境変数が落ちる。Tauri の `DEP_TAURI_CORE:WINDOW__..._PERMISSION_FILES_PATH` が届かず、ugoe-studio・ugoe-editor のビルドスクリプトが `Permission core:window:allow-start-dragging not found` などで失敗した (最小の再現で確かめた)。リポジトリの直下に `.mbx.toml` (`build_script_execution = false`) を足して回避した (`crates:` に無いルートの設定ファイル)。この設定はビルドスクリプトのコンパイル時に効くので、**main の作業ツリーや既存の worktree の managed target で Tauri のアプリが同じ失敗をしたら、一度 `mbx clean` する。**
- 依頼文の前提との違い: 「mbx を通さない cargo は ./target を使うので混ざらない」は正しくなかった。mbx は `target` を managed target へのシンボリックリンクにする (main の `target` もすでにリンク) ので、素の cargo も同じ場所を使う。実体の `target/` があれば mbx が最初に通ったときに取り込む。素の cargo を同じ target で動かしても、試した範囲 (`cargo build -p ugoe-model`・`cargo test -p ugoe-runtime`) では失敗しなかったが、読み取り専用のハードリンクの成果物を書き直す場面では失敗しうる (docs に `mbx clean` での直し方を書いた)。
- 計測 (WSL2・ext4・並行するワーカー無し):
  - mbx が有効な `just wt-add` (T9998 で試した): 約 1 秒 (git worktree と pnpm だけ)。従来は 1 つ 2〜5 分・約 30GB。
  - T0018 (target/ 無し) の最初の `just crate ugoe-runtime`: 14 秒 (27 hits / 0 misses)。T9998 では 3 秒 (49 hits / 0 misses)。
  - 最初の `just ci`: T0018 で 297 秒 (置き場は main の 1 回のビルド分だけ。test で 140 hits)。その後に作った T9998 で 195 秒 (clippy 744 hits / 31 misses、test 675 hits / 35 misses)。
  - worktree ごとの managed target は約 25GB で、うち置き場へのハードリンクが 8GB、worktree に固有のもの (ワークスペースのクレートとテストの実行ファイル) が 16.6GB。コピーの 30GB よりは小さいが、ゼロにはならない。
- CI には入れないと決めた (理由は docs/workflow/build-cache.md#ci-には入れない。ランナーをまたぐ共有にはリモートのキャッシュの置き場と資格情報が要り、依存クレートは rust-cache で持ち越している)。
- やらなかったこと: macOS・Windows では確かめていない (justfile の PATH の追加は Unix のみ。macOS の shim の場所は `data_local_directory()` = `~/Library/Application Support/mbx/bin` のはずだが未確認)。mise の wrapper で包んだ場合の判定はしない (コピーする側に倒れる)。`just app::build` (scripts/app.just のモジュール) にはルートの `export PATH` が届くか確かめていない。
- 起票の候補:
  - mbx へ不具合を報告する: ビルドスクリプトのラッパーが `/bin/sh` (dash) を通るため、`:` を含む `DEP_*` の環境変数が落ちる (再現: `links` を持つクレートが `cargo:core:window__PATH=/x` を出し、依存するクレートのビルドスクリプトで `DEP_*` を表示する)。直ったら `.mbx.toml` を外す。
  - worktree ごとの固有の成果物 (約 16GB) を減らす: dev のデバッグ情報を減らす (`CARGO_PROFILE_DEV_DEBUG=0` や `split-debuginfo`) を開発の機械でも使うか検討する。

