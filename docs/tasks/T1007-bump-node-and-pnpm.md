---
id: T1007
title: "Node.js を v26.11.1、pnpm を v12.12.1 に上げる"
status: in-progress
milestone: Release
crates: [package.json, pnpm-lock.yaml, .tool-versions, apps, packages, bindings/web, tests, scripts, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T1007
---

# T1007: Node.js と pnpm を上げる

## 目的

`just tool-updates` (T1004) が Node.js v26.11.1 と pnpm v12.12.1 を知らせた。利用者の決定 (2026-10-12): 上げる。

## 受け入れ条件

- [ ] `.tool-versions`・`package.json` の `packageManager`・`engines`・ワークフローの setup-node と pnpm の版など、T1004 の `PINNED_TOOLS` の写しをすべて新しい版にした
- [ ] pnpm の版を上げて lockfile の形式が変わるなら作り直し、依存の解決が変わっていないこと (差分が lockfile の形式だけ) を確かめた。変わったなら理由を書いた
- [ ] Node.js のメジャーの版が変わるなら、使っている API の非推奨・挙動の違い (ESM・`node:test`・fetch など) を確かめた
- [ ] `just ci`・`just e2e` (アプリごとに `--only`)・`just ui-audit` (アプリごとに `--only`)・`just perf` (wasm の側)・`just licenses` が通る
- [ ] 手元の機械で入れ直す手順 (mise / asdf など) を docs に書いた

## 参考

- T1004 のメモ、.tool-versions、package.json、.github/workflows、docs/release.md#固定した道具の版

## メモ

- 利用者の決定 (2026-10-12) を受けてオーケストレーターが起票した。
