---
id: T1007
title: "Node.js を v26.11.1、pnpm を v12.12.1 に上げる"
status: done
milestone: Release
crates: [package.json, pnpm-lock.yaml, .tool-versions, apps, packages, bindings/web, tests, scripts, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T1007
---

# T1007: Node.js と pnpm を上げる

## 目的

`just tool-updates` (T1004) が Node.js v26.11.1 と pnpm v12.12.1 を知らせた。利用者の決定 (2026-10-12): 上げる。

## 受け入れ条件

- [x] `.tool-versions`・`package.json` の `packageManager`・`engines`・ワークフローの setup-node と pnpm の版など、T1004 の `PINNED_TOOLS` の写しをすべて新しい版にした
- [x] pnpm の版を上げて lockfile の形式が変わるなら作り直し、依存の解決が変わっていないこと (差分が lockfile の形式だけ) を確かめた。変わったなら理由を書いた
- [x] Node.js のメジャーの版が変わるなら、使っている API の非推奨・挙動の違い (ESM・`node:test`・fetch など) を確かめた
- [x] `just ci`・`just e2e` (アプリごとに `--only`)・`just ui-audit` (アプリごとに `--only`)・`just perf` (wasm の側)・`just licenses` が通る
- [x] 手元の機械で入れ直す手順 (mise / asdf など) を docs に書いた

## 参考

- T1004 のメモ、.tool-versions、package.json、.github/workflows、docs/release.md#固定した道具の版

## メモ

- 利用者の決定 (2026-10-12) を受けてオーケストレーターが起票した。
- やったこと (claude-worker-T1007):
  - `.tool-versions` の nodejs を 26.11.1、`package.json` の `packageManager` を pnpm@12.12.1 にした。`PINNED_TOOLS` の Node.js と pnpm は
    `mirrors` が空で、ワークフローは `node-version-file: .tool-versions` と `pnpm/action-setup` (版は `packageManager` から読む) なので、
    ほかに写した場所は無い。`tools/spec-check/package.json` の `engines` (`node >=24`) は下限なので変えていない。
  - `pnpm-lock.yaml` の差分は先頭の文書の `packageManagerDependencies` (pnpm 自身と `@pnpm/exe.*` の版と integrity) だけ。依存の解決を書いた
    2 つ目の文書 (`lockfileVersion: '9.0'`) は変わっていない。`pnpm install` は「Already up to date」。
  - Node.js はメジャー (26) のままのマイナーとパッチの更新なので、非推奨・挙動の違いの調べは要らない (検査とテストがすべて通った)。
  - docs/release.md の「版を上げる」に「手元の Node.js と pnpm を入れ直す」(asdf・mise。Node.js 25 から同梱されない corepack を版ごとに入れる) を足した。
  - 新しい版で `just ci`・`just e2e --only editor|viewer|studio`・`just ui-audit --only editor|viewer|studio` (エラー 0)・`just perf-wasm`
    (update・follow の中央値 17〜57µs / 上限 300µs)・`just licenses` が通った。
- やらなかったこと: CI の上での確認 (push しないため)。mise での入れ直しの手順は書いただけで、この機械 (asdf) では流していない。
- 起票の候補: なし。

