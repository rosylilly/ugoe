---
id: T1004
title: "版を固定した道具 (gitleaks・cargo-about・git-cliff・actionlint など) の新しい版に気づく仕組みを足す"
status: todo
milestone: Release
crates: [scripts, justfile, .github/workflows, docs]
depends_on: []
assignee:
---

# T1004: 固定した道具の新しい版に気づく

## 目的

gitleaks (T0098。版と sha256)・cargo-about (T0005)・git-cliff (T0086)・ワークフローで入れる道具 (cargo-fuzz・wasm-bindgen-cli など) は、版を手で固定している。Dependabot はこれらを見ないので、新しい版 (特に脆弱性の修正) に気づけない。

## 受け入れ条件

- [ ] 版を固定している道具を洗い出し、一覧 (道具・固定している場所・版・上流の取得先) を 1 か所にまとめた (重複していれば寄せる)
- [ ] 上流 (GitHub の releases・crates.io) の最新の版と比べ、新しい版があれば知らせる `just tool-updates` を足した。ネットワークに触れるので `just ci` には入れない。比べるだけで、版を書き換えない
- [ ] 定期の重い検査 (release-checks.yml) か別の定期のワークフローで流し、新しい版があれば失敗ではなくジョブの要約 (`$GITHUB_STEP_SUMMARY`) に出す (課金の分を増やしすぎないよう、軽いジョブにする)
- [ ] 版を上げる手順 (sha256 の取り直しなど) を docs/release.md か docs/testing.md に書いた。GitHub Actions の action 自体の版は Dependabot (github-actions) で上げられるかを確かめ、使うなら設定を足すか後続の候補に書いた
- [ ] 単体テスト (版の比べ方・一覧の読み取り)。actionlint が通る。`just ci` が通る

## 参考

- T0098・T0005・T0086 のメモ、scripts/secrets.mjs・changelog.mjs・justfile の `cargo-about`、.github/workflows

## メモ

- 元の候補: T0098 (gitleaks の新しい版に気づく仕組み)。対象を固定した道具全体に広げた。オーケストレーターが起票した (2026-10-12)。
