---
id: T1662
title: "エディタ: 送る枠の行末を --ugoe-scrollbar-gap にし、tokens.test.ts の検査の対象にエディタを足す"
status: todo
milestone: M10
crates: [apps/editor, packages/ui, docs]
depends_on: []
assignee:
---

# T1662: エディタ: 送る枠の行末を --ugoe-scrollbar-gap にし、tokens.test.ts の検査の対象にエディタを足す

## 目的

WebKitGTK のスクロールバーは幅を取らずに中身の上に重ねて描き、指を乗せると 6〜14px に太る (T0970)。T0970 はビューアとスタジオの送る枠の
行末 (右) の padding を `--ugoe-scrollbar-gap` (16px) にしたが、エディタ (範囲の外) は直していない。エディタのパネルの送る枠 (`.pane`・`aside`) は
右 12px で、指を乗せたときだけ 2px 掛かる。ほかにも空けていない枠がある。

## 受け入れ条件

- [ ] エディタの縦に送る枠の行末を `--ugoe-scrollbar-gap` にした。少なくとも T0970 の一覧: パネルの `.pane`・`aside`、PickList、GluesPanel の組の一覧、
      LightingPanel のメッシュの一覧、MotionEditor、ExpressionEditor の表、FileMenu の警告の一覧、Viewport の空の案内、LicenseDialog。
      空けない枠は、理由をスクロールバーの言葉で CSS のコメントに書いた
- [ ] packages/ui の `tokens.test.ts` の検査 (送る枠の規則が `--ugoe-scrollbar-gap` を使うか、使わない理由を書く) の対象にエディタを足し、
      空けていない規則が残っていると落ちることを確かめた
- [ ] 狭くなった中身で文字が省略・折り返しにならないかを `just ui-audit --only editor` (600 秒に収まらなければ `--variants` で分けて、すべての組) と
      `--compare` で確かめ、増えた指摘が無い
- [ ] docs/ui.md (レイアウトの「送り」) を直した。`just e2e --only editor` と `just ci` が通る
- 確かめていないこと (WebView2・WKWebView の重ねて描くスクロールバー) はメモに書く

## 参考

- T0970 のメモ (やったこと・やらなかったこと・起票の候補)、docs/ui.md (`--ugoe-scrollbar-gap`)
- apps/editor/src (App.svelte の `.pane`・app.css の `aside`・上の一覧の部品)、packages/ui/src/tokens.test.ts

## メモ

- T0052 で起票した (元の候補: T0970 (エディタの送る枠に `--ugoe-scrollbar-gap`))。
- 作業中の T1654 (TreeItem.svelte) とは別のファイル。ツリーのパネルの送る枠を触るなら T1654 の後に rebase して確かめる。
