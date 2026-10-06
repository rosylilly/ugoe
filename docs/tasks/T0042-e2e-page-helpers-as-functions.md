---
id: T0042
title: "e2e: page.mjs のページ側のコードを文字列でなく関数で渡す"
status: in-progress
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

- [ ] `page.mjs` のページ側のコード (部品を名前で探す・値を読むなど) が関数として書かれ、`session.execute` / `session.run` には関数から作った本体を渡す
      (引数の渡し方をメモに書く)。文字列の中の正規表現・エスケープが残っていない
- [ ] `just fmt` (Biome) を流しても page.mjs の正規表現が変わらない
- [ ] `just e2e` (Linux の WebKitGTK。3 つのアプリ) がすべて通る
- [ ] tests/e2e/README.md (ページ側の道具の書き方) を直した
- [ ] `just ci` が通る

## 参考

- T0847 のメモ (起票の候補)
- tests/e2e/lib/page.mjs、tests/e2e/README.md

## メモ

- T0041 で起票した (元の候補: T0847 (page.mjs のページ側のコード))。
