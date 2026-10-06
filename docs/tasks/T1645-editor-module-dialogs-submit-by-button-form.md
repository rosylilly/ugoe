---
id: T1645
title: "エディタ: 新しいモジュールとモジュールの設定のダイアログの決定のボタンを、Button の form で送る形にする"
status: done
milestone: M10
crates: [apps/editor]
depends_on: []
assignee: claude-worker-T1645
---

# T1645: エディタ: 新しいモジュールとモジュールの設定のダイアログの決定のボタンを、Button の form で送る形にする

## 目的

T0953 で `@ugoe/ui` の `Button` に `form` の口が入り、エディタのほかのダイアログはそれを使う形になった。`NewModuleDialog`・`ModuleSettingsDialog`
は、`<form>` の `onsubmit` と決定のボタンの `onclick` が同じ関数を呼ぶ形のまま残っている (T0953 の候補)。送る道を 1 つにそろえる。

## 受け入れ条件

- [x] 2 つのダイアログの決定のボタンが `type="submit"` (`form` の口) で `<form>` を送り、`onclick` で同じ関数を呼ばない。Enter と決定のボタンの両方で 1 回だけ送る
- [x] 部品のテスト (Enter で送る・ボタンで送る・送れないとき (名前が不正・スロットが無い) は送らない・送るのは 1 回だけ)
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0953 のメモ (やらなかったこと・起票の候補)
- apps/editor/src/lib/NewModuleDialog.svelte・ModuleSettingsDialog.svelte、packages/ui (`Button` の `form`)

## メモ

- T0043 で起票した (元の候補: T0953 (`NewModuleDialog`・`ModuleSettingsDialog` の決定のボタン))。

- **やったこと:**
  - `NewModuleDialog`・`ModuleSettingsDialog` の `<form>` に `$props.id()` から作った id を付け、操作の並びの決定のボタン (「Create」・「Apply」) を
    `type="submit"`・`form={id}` にして `onclick` を外した。決定のボタンが form の既定のボタンになるので、名前の欄の Enter はそれを押したのと
    同じ道で送り、ボタンが押せない (名前が不正・スロットが無い・送っている途中) あいだは Enter でも送らない (前は送ってから関数の中で弾いていた)。
    form を置く理由のコメント (style の前) もこの形に書き直した。
  - 部品のテスト `apps/editor/src/lib/ModuleDialogs.test.ts` (7 件): ボタンが form を送る形であること・ボタンで 1 回だけ送る・Enter で 1 回だけ送る・
    名前が不正なら送らない (両方のダイアログ)・スロットの無い本体では送らない (新しいモジュール)。happy-dom は Enter で送らないので、
    Enter は implicit submission の規則 (既定のボタンがあればそれで送り、押せなければ送らない) を `requestSubmit` で再現した。
    変更前のコードではボタンと Enter の 4 件が落ちることを確かめた。
  - 文書: docs/editor.md#ダイアログ の「Enter」の行に、決定のボタンが `form` で送ることと押せないあいだは Enter でも送らないことを足した
    (`crates:` の外だが、変えた振る舞いの説明だけの小さな同期)。
- **検査:** `just e2e --only editor` は 15 件すべて通った。`just ci` は通った。
- **やらなかったこと:** 実物の WebView (WebKitGTK・WebView2・WKWebView) で Enter の送信を確かめてはいない (happy-dom の部品のテストだけ)。
  `just ui-audit` は流していない (見た目・並び・クラス名は変えていない)。
- **起票の候補:** なし。
