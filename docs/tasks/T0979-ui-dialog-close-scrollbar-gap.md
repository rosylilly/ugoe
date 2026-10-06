---
id: T0979
title: "@ugoe/ui: Dialog の閉じるボタンを、重ねて描くスクロールバーから離す"
status: todo
milestone: M10
crates: [packages/ui, tests, docs]
depends_on: []
assignee:
---

# T0979: @ugoe/ui: Dialog の閉じるボタンを、重ねて描くスクロールバーから離す

## 目的

T0055 で足した ui-audit の `scrollbar` の検査で、エディタの 200% の拡大の設定のダイアログ (`*-zoom200-settings-ai`) の閉じるボタン
(`.ugoe-dialog__close`) が指摘として残った。ダイアログの全体が縦に送るときだけ、ボタンの右が枠の右端から `space-4` (12px) しか空いておらず、
指を乗せて太ったスクロールバー (WebKitGTK・macOS で 14px まで) が 2px 掛かる。決め (docs/ui.md#レイアウト の「送り」) は、
縦に送る枠の行末を `--ugoe-scrollbar-gap` 空けること。

## 受け入れ条件

- [ ] packages/ui の Dialog で、閉じるボタンの右を `--ugoe-scrollbar-gap` 以上空ける。ダイアログが送るときだけ空ける
      (`scrollGap.ts` の `markScrolling` の `data-scrolling` などで) か、いつも空けるかを決め、理由をメモと docs/ui.md に書いた
      (いつも空けるとすべてのダイアログの見た目が変わる。コンパクトの密度でも見る)
- [ ] 振る舞いを変えたらテスト (packages/ui の components.test.ts など) を書いた
- [ ] `just ui-audit --only editor` (少なくとも `zoom200` の組) で `scrollbar` の `.ugoe-dialog__close` の指摘が消え、ほかの指摘が増えていない
      (直す前と `--compare` で比べる)。ビューア・スタジオのダイアログも `--quick` で見た
- [ ] tests/ui-audit/README.md の「重ねて描くスクロールバー」の「残る指摘 (直していないもの)」を今の形にした
- [ ] 画面の部品・クラス名を変えたら 3 つのアプリの `just e2e` を流した。`just ci` が通る

## 参考

- T0055 のメモ (直していない指摘)、tests/ui-audit/README.md の「重ねて描くスクロールバー」
- packages/ui/src/components/Dialog.svelte (`.ugoe-dialog__close`)、packages/ui/src/scrollGap.ts (T0976 の一覧の送り)
- docs/ui.md#レイアウト の「送り」(T0970)

## メモ

- T0063 で起票した (元の候補: T0055 (Dialog の閉じるボタンの空き))。
