---
id: T0847
title: "e2e: スタジオの API とブラウザソースの手順が、T0817 で消えた .studio__api を待って時間切れになる"
status: done
milestone: M10
crates: [tests/e2e, apps/studio, docs]
depends_on: []
assignee: claude-worker-T0847
---

# T0847: e2e: スタジオの API とブラウザソースの手順が、T0817 で消えた .studio__api を待って時間切れになる

## 目的

T0846 の作業中に、e2e のスタジオの手順のうち「connects a client to the studio API」と「serves the browser source」が、
もう無い `.studio__api` を待って時間切れになることが分かった (T0846 の候補)。T0817 で操作ウィンドウのツールバーの API・ソースの札を
「いまの状態」の印 (`StatusBadge`) に移したためと見込まれる。main でも失敗するはず (確かめていない)。

## 受け入れ条件

- [x] main で `just e2e --only studio` を流し、失敗を再現した (失敗した手順と待っていたものをメモに)
- [x] 2 つの手順を、今の「いまの状態」の印 (API・ソース) を役割と名前で探す形に直した (クラス名に頼らない)
- [x] `just e2e --only studio` がすべて通る。tests/e2e/README.md の筋書きを直した
- [x] `just ci` が通る

## 参考

- T0817・T0846 のメモ、apps/studio/src/StatusBadge.svelte・lib/status.ts、tests/e2e/specs/studio.test.mjs

## メモ

- オーケストレーターが直接起票した (T0846 の候補。回帰なので候補の一覧を待たずに起票)。
- 振り返り: T0817 は e2e を流していなかった (報告で「e2e は流していない」としていた)。UI の並びを変えるタスクは e2e も流すよう、ワーカーへの指示に足す。
- 再現 (main と同じコードの worktree で `just e2e --only studio`): 13 手順のうち 2 つが失敗。
  「connects a client to the studio API」は `__e2e.byText(".studio__api", "^API$")` を、「serves the browser source」は
  `__e2e.byText(".studio__api", "^Source$")` を待って、どちらも `timed out after 10000ms waiting for the API badge / the source badge`。
  T0817 で札が「いまの状態」の印 (`<ul aria-label="Status">` の `<li>`、文は「API on」「Source on」) に移ったため。
- やったこと: `tests/e2e/lib/page.mjs` に `__e2e.listItem(listName, pattern)` を足した (名前 (aria-label・aria-labelledby) が合うリスト
  (`ul`・`ol`・`role=list`) の中で、文字が合う項目)。2 つの手順は `listItem("^Status$", "^API on$")`・`listItem("^Status$", "^Source on$")` を待つ。
  tests/e2e/README.md の筋書き (スタジオ) と「筋書きを足す」(アプリのクラスに頼らず、リストの項目は役割と名前で探す) を直した。
  `just e2e --only studio` は 13 手順すべて通った (2 回)。`just ci` も通った。apps/studio と docs/ (README を除く) は変えていない。
- 気づいたこと: Biome の自動修正 (pre-commit) が、ページに渡すテンプレート文字列の中の `/\\s+/` を不要なエスケープとみなして `/s+/` に書き換え、
  正規表現が壊れた。`/ +/` に変えて避けた。`page.mjs` の既存の `text` (`/\\s+/g`) は書き換えられていない (なぜ片方だけかは調べていない)。
- 起票の候補: page.mjs のページ側のコードを文字列でなく関数 (`session.execute` に関数の本体を渡す) にして、Biome の書き換えと
  エスケープの二重化を避ける。
