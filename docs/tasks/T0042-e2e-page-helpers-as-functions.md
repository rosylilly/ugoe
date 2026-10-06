---
id: T0042
title: "e2e: page.mjs のページ側のコードを文字列でなく関数で渡す"
status: done
milestone: M9
crates: [tests/e2e, docs]
depends_on: []
assignee: claude-worker-T0042
---

# T0042: e2e: page.mjs のページ側のコードを文字列でなく関数で渡す

## 目的

`tests/e2e/lib/page.mjs` は、ページの中で動かすコードを文字列 (テンプレートリテラル) で `session.execute` に渡している。文字列の中の正規表現は
エスケープが二重になり (`/\\s+/`)、Biome の整形が書き換えて (`/\\s+/` → `/s+/`) 意味が変わることがある (T0847 の候補。バグ寄り)。
ページ側のコードを関数として書き、関数の本体を渡す形にして、エスケープの二重化と整形による書き換えをなくす。

## 受け入れ条件

- [x] `page.mjs` のページ側のコード (部品を名前で探す・値を読むなど) が関数として書かれ、`session.execute` / `session.run` には関数から作った本体を渡す
      (引数の渡し方をメモに書く)。文字列の中の正規表現・エスケープが残っていない
- [x] `just fmt` (Biome) を流しても page.mjs の正規表現が変わらない
- [x] `just e2e` (Linux の WebKitGTK。3 つのアプリ) がすべて通る
- [x] tests/e2e/README.md (ページ側の道具の書き方) を直した
- [x] `just ci` が通る

## 参考

- T0847 のメモ (起票の候補)
- tests/e2e/lib/page.mjs、tests/e2e/README.md

## メモ

- T0041 で起票した (元の候補: T0847 (page.mjs のページ側のコード))。
- やったこと: `lib/webdriver.mjs` の `Session.execute(script, ...args)` が関数も受け取るようにした。関数なら
  `return (<関数のソース>).apply(null, arguments);` を本体としてページに送る (引数の渡し方: WebDriver の `args` がそのまま関数の引数になる。
  JSON にできる値と要素。Node の側の変数は閉じ込められないので、要るものは引数で渡す)。`session.run` はもとから関数を受け取る。
  `lib/page.mjs` の `window.__e2e` を `pageHelpers()`、invoke を数える仕組みを `watchInvokes(commands)` という関数に書き直し、
  `invokeStats` も関数で渡す。`webdriver.mjs` の `waitFindText`・`clickText` (文字列の中に `/\\s+/` があった) も関数にした。
  page.mjs・webdriver.mjs に文字列の中の正規表現は残っていない。`listItem` の `aria-labelledby` の分割を `/ +/` から `/\s+/` にした
  (ID の区切りは空白文字。ほかの振る舞いは同じ)。
- `just fmt` を 2 回流して page.mjs の正規表現 (`/\s+/g`・`/\s+/`) が変わらないことを確かめた。
- `just e2e` (3 つのアプリ) は editor 15・viewer 6・studio 13 手順すべて通った。`just ci` も通った。
- やらなかったこと: `lib/tracking.mjs` の `trackerState` などの文字列のスクリプト (正規表現もエスケープも無い) はそのまま。
- 起票の候補: なし。
