---
id: T0976
title: "@ugoe/ui: Select・Menu の一覧が送るときだけ、行末をスクロールバーの分空ける"
status: todo
milestone: M10
crates: [packages/ui, docs]
depends_on: [T1662]
assignee:
---

# T0976: @ugoe/ui: Select・Menu の一覧が送るときだけ、行末をスクロールバーの分空ける

## 目的

WebKitGTK のスクロールバーは中身の上に重ねて描く (T0970)。T0970 は送る枠の行末に `--ugoe-scrollbar-gap` (16px) を空ける決めにしたが、
@ugoe/ui の `Select`・`Menu` の一覧 (padding 2px) は直さなかった。低い窓でしか送らず、いつも空けるとすべてのメニューが 14px 広がるため。
一覧が送るときだけ、行末 (行の右の要素。ショートカットやチェック) がスクロールバーに隠れないようにする。

## 受け入れ条件

- [ ] `Select`・`Menu` の一覧が縦に送るとき (中身が枠より高いとき) だけ、行末を `--ugoe-scrollbar-gap` 空ける。送らないときの幅と見た目は変わらない
      (送るかどうかで padding を変えるか、行の右の要素の側で空けるかを決め、理由をメモに書いた。大きさが変わったら測り直す)
- [ ] 部品のテスト: 送るときに行末を空け、送らないときは空けない (happy-dom は大きさを測らないので、大きさを差し替えるか `just ui-shots` の見本で確かめる)
- [ ] packages/ui の `tokens.test.ts` の検査 (送る枠は `--ugoe-scrollbar-gap` を使うか理由を書く) が、この 2 つの部品でも通る
- [ ] docs/ui.md (レイアウトの「送り」・部品の一覧) と packages/ui/README.md を直した
- [ ] `just ui-shots`・`just ui-audit` (3 つのアプリ。600 秒に収まらなければ分けて) で増えた指摘が無い。`just ci` が通る

## 参考

- T0970 のメモ (やらなかったこと・起票の候補)、docs/ui.md (`--ugoe-scrollbar-gap`)
- packages/ui/src/components (Select・Menu)、packages/ui/src/tokens.test.ts

## メモ

- T0052 で起票した (元の候補: T0970 (Select・Menu が送るときだけ行末を空ける))。
- 同じ tokens.test.ts を触る T1662 (エディタの送る枠) の後にした。
