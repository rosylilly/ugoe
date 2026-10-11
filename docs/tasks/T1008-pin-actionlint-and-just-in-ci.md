---
id: T1008
title: "actionlint を固定して CI で回し、CI の just を .tool-versions の版にそろえる。Dependabot は公開後に入れると書く"
status: done
milestone: Release
crates: [.tool-versions, scripts, justfile, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T1008
---

# T1008: actionlint と just の版を CI でそろえる

## 目的

T1004 の所見: actionlint はどこにも版が固定されておらず CI でも回していない (作業者が手元で流しているだけ)。CI の setup-just は最新の just を入れていて、手元の `.tool-versions` と食い違いうる。

## 決まっていること (利用者、2026-10-12)

- Dependabot は公開の後 (PR の運用に移った後) に、github-actions・cargo・npm をまとめて入れる。今は入れない。

## 受け入れ条件

- [x] actionlint の版を固定し (`.tool-versions` か、gitleaks と同じく版と sha256 を固定して取得する形)、`just actionlint` を足して push ごとの軽い CI で回す。shellcheck も版を固定する (手元の shellcheck の出力を actionlint が読めない問題 (T1004 のメモ) を避ける)
- [x] actionlint と shellcheck を T1004 の `PINNED_TOOLS` に入れた
- [x] CI の setup-just に `.tool-versions` の just の版を渡すようにした (写しの検査にも入れる)
- [x] docs/release.md の公開の前の点検 (PR の運用へ移る段) に「Dependabot (github-actions・cargo・npm) を入れる」を足した
- [x] `just ci` が通る (actionlint を入れるなら、今のワークフローがすべて通ること)

## 参考

- T1004 のメモ、scripts/tool-updates.mjs・secrets.mjs、.github/workflows、docs/release.md

## メモ

- 元の候補: T1004 (actionlint の固定・setup-just の版・Dependabot)。オーケストレーターが起票した (2026-10-12)。
- やったこと (2026-10-12):
  - `scripts/actionlint.mjs` と `just actionlint`: actionlint 1.7.12 と shellcheck 0.11.0 を、gitleaks と同じく版と sha256 を固定して
    GitHub の Release からメインの作業ツリーの `target/tools/` に取得し、`-shellcheck <固定した shellcheck> -pyflakes=` で流す
    (手元の actionlint・shellcheck・pyflakes は使わない)。actionlint の sha256 は Release の checksums.txt、shellcheck は Release に
    checksums が無いので GitHub の API の `digest` から写した。取得・検証・展開は `scripts/lib/release-tool.mjs` に出し、`scripts/secrets.mjs` もそれを使う。
  - `just ci` と ci.yml の `rust-checks` (push・PR ごと) に `just actionlint` を足した。今のワークフローはすべて指摘なしで通る
    (shellcheck 0.11.0 でも。わざと `echo $FOO` を書いたワークフローで SC2086 が出ることも確かめた)。
  - `PINNED_TOOLS` に actionlint と shellcheck を足した (写しは同じファイルの Release のファイル名)。
  - setup-just を使う 10 のワークフローの 17 か所に `just-version: 1.58.0` を足し、`PINNED_TOOLS` の just の `mirrors` にした。
    setup-just を使うのに版を渡さないワークフロー・一覧に無いワークフローがあると落ちる単体テストを `scripts/tool-updates.test.mjs` に足した。
  - docs/release.md: 公開の前の点検の「PR の運用へ移る」に Dependabot (github-actions・cargo・npm) を入れることを足し、固定した道具の版の節
    (一覧・CI の just・上げ方の表・Dependabot の段落) を直した。docs/testing.md の検査の表と rust-checks の行にも足した。
  - `.github/dependabot.yml` は作っていない (利用者の決定)。
- 確かめていないこと: GitHub Actions の上での実際の run (push と workflow_dispatch はしていない)。macOS・Windows・arm64 での取得と展開
  (sha256 は上流の値を写しただけで、取得して確かめたのは linux-x64 と、展開の中身の配置を見た Windows の zip・darwin-arm64 の tar.gz)。
- 範囲の外: AGENTS.md のコマンドの表には `just actionlint` を足していない (`crates:` の外)。
- 起票の候補:
  - AGENTS.md のコマンドの表に `just actionlint` を足す。
