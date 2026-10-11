---
id: T1008
title: "actionlint を固定して CI で回し、CI の just を .tool-versions の版にそろえる。Dependabot は公開後に入れると書く"
status: in-progress
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

- [ ] actionlint の版を固定し (`.tool-versions` か、gitleaks と同じく版と sha256 を固定して取得する形)、`just actionlint` を足して push ごとの軽い CI で回す。shellcheck も版を固定する (手元の shellcheck の出力を actionlint が読めない問題 (T1004 のメモ) を避ける)
- [ ] actionlint と shellcheck を T1004 の `PINNED_TOOLS` に入れた
- [ ] CI の setup-just に `.tool-versions` の just の版を渡すようにした (写しの検査にも入れる)
- [ ] docs/release.md の公開の前の点検 (PR の運用へ移る段) に「Dependabot (github-actions・cargo・npm) を入れる」を足した
- [ ] `just ci` が通る (actionlint を入れるなら、今のワークフローがすべて通ること)

## 参考

- T1004 のメモ、scripts/tool-updates.mjs・secrets.mjs、.github/workflows、docs/release.md

## メモ

- 元の候補: T1004 (actionlint の固定・setup-just の版・Dependabot)。オーケストレーターが起票した (2026-10-12)。
