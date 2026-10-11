---
id: T1004
title: "版を固定した道具 (gitleaks・cargo-about・git-cliff・actionlint など) の新しい版に気づく仕組みを足す"
status: done
milestone: Release
crates: [scripts, justfile, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T1004
---

# T1004: 固定した道具の新しい版に気づく

## 目的

gitleaks (T0098。版と sha256)・cargo-about (T0005)・git-cliff (T0086)・ワークフローで入れる道具 (cargo-fuzz・wasm-bindgen-cli など) は、版を手で固定している。Dependabot はこれらを見ないので、新しい版 (特に脆弱性の修正) に気づけない。

## 受け入れ条件

- [x] 版を固定している道具を洗い出し、一覧 (道具・固定している場所・版・上流の取得先) を 1 か所にまとめた (重複していれば寄せる)
- [x] 上流 (GitHub の releases・crates.io) の最新の版と比べ、新しい版があれば知らせる `just tool-updates` を足した。ネットワークに触れるので `just ci` には入れない。比べるだけで、版を書き換えない
- [x] 定期の重い検査 (release-checks.yml) か別の定期のワークフローで流し、新しい版があれば失敗ではなくジョブの要約 (`$GITHUB_STEP_SUMMARY`) に出す (課金の分を増やしすぎないよう、軽いジョブにする)
- [x] 版を上げる手順 (sha256 の取り直しなど) を docs/release.md か docs/testing.md に書いた。GitHub Actions の action 自体の版は Dependabot (github-actions) で上げられるかを確かめ、使うなら設定を足すか後続の候補に書いた
- [x] 単体テスト (版の比べ方・一覧の読み取り)。actionlint が通る。`just ci` が通る

## 参考

- T0098・T0005・T0086 のメモ、scripts/secrets.mjs・changelog.mjs・justfile の `cargo-about`、.github/workflows

## メモ

- 元の候補: T0098 (gitleaks の新しい版に気づく仕組み)。対象を固定した道具全体に広げた。オーケストレーターが起票した (2026-10-12)。
- やったこと (2026-10-12):
  - 一覧は `scripts/tool-updates.mjs` の `PINNED_TOOLS` の 1 か所。12 の道具 (gitleaks・cargo-about・git-cliff・cbindgen・cargo-ndk・
    tauri-driver・artifact-signing-cli・Godot・Rust・Node.js・pnpm・just) ごとに、版を書いた元の場所 (`pin`)・同じ版を写した場所
    (`mirrors`)・上流 (GitHub の Release か crates.io) を持つ。`just tool-updates --list` で表を出す。
  - 重複の寄せ方: 写しを消すのではなく、写した場所をすべて一覧に書き、元と食い違えば単体テスト (`just script-check`、`just ci` に入る)
    と `--list` が落ちる形にした。cbindgen は元 (`scripts/ffi.mjs`) のほかに 3 つのワークフロー・justfile・crates/ugoe-ffi/README.md に、
    git-cliff は docs/release.md に、tauri-driver は docs/testing.md に、gitleaks は同じファイルのコメントの URL に写しがある。
  - `just tool-updates`: 上流と比べて表を出すだけ (版は書き換えない。新しい版があっても上流を読めなくても終了コード 0。写しの食い違いだけ 1)。
    タグは版として数で比べる (Release の並びの順は使わない。Godot は 3.x の修正が先頭に来る)。先行版・下書き・crates.io の取り下げは除く。
    Node.js のために「固定したメジャー版の中の最新」も出す。
  - release-checks.yml に `tool-updates` のジョブ (ubuntu・checkout と setup-node だけ・5 分の上限) を足した。表を `$GITHUB_STEP_SUMMARY` に
    書き、新しい版ごとに `::notice::` を出す。上流は main と無関係に進むので `gate` を待たない。
  - 上げ方は docs/release.md#固定した道具の版 に道具ごとの表で書いた (gitleaks の sha256・Godot の SHA512 の取り直しなど)。
    docs/testing.md の release-checks の表にもジョブを足した。
  - 確かめたこと: 手元で `GH_TOKEN=$(gh auth token) just tool-updates` を流し、Godot 4.7.2-stable・Rust 1.99.0・Node.js v26.11.1・
    pnpm v12.12.1 が新しいと出た (2026-10-12。版は上げていない)。`GITHUB_ACTIONS`・`GITHUB_STEP_SUMMARY` を付けて要約と notice の
    出力も確かめた。actionlint 1.7.12 (`-shellcheck=`。手元の shellcheck 0.9.0 との連携は JSON を読めず失敗する既知の問題) で全ワークフローが通る。
    `just ci` が通る。
  - 確かめていないこと: GitHub Actions の上での実際の run (push と workflow_dispatch はしていない)。
- Dependabot (github-actions): Dependabot の version updates の `github-actions` は、ワークフローと composite action の `uses:` の
  メジャー版のタグ (`actions/checkout@v7` など) を新しいメジャー版へ上げる PR を作れる。`.github/dependabot.yml` は今は無く、置くと
  PR が自動で作られるので、置くかは利用者の判断として足さなかった (docs/release.md にも書いた)。
- やらなかったこと: actionlint はリポジトリのどこにも版を固定しておらず (手元の asdf の 1.7.12 を手で使うだけ)、CI でも流していないので
  一覧に入れなかった。cargo-fuzz・cargo-xwin は版を固定していない。wasm-bindgen-cli は Cargo.lock の依存に従うので対象外。
- 起票の候補:
  - `.github/dependabot.yml` に `github-actions` (週 1 回・まとめた 1 つの PR) を足すか、利用者に決めてもらう。
  - actionlint を `.tool-versions` で固定し、ワークフローを変えた PR で流すジョブ (か `just ci` の任意の検査) を足す (足したら PINNED_TOOLS にも入れる)。
  - CI の just (`extractions/setup-just`) に `.tool-versions` の版を渡して手元とそろえる。
  - 今出ている新しい版 (Godot 4.7.2・Rust 1.99.0・Node.js 26.11.1・pnpm 12.12.1) を上げる。

