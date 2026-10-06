---
id: T1645
title: "エディタ: 新しいモジュールとモジュールの設定のダイアログの決定のボタンを、Button の form で送る形にする"
status: todo
milestone: M10
crates: [apps/editor]
depends_on: []
assignee:
---

# T1645: エディタ: 新しいモジュールとモジュールの設定のダイアログの決定のボタンを、Button の form で送る形にする

## 目的

T0953 で `@ugoe/ui` の `Button` に `form` の口が入り、エディタのほかのダイアログはそれを使う形になった。`NewModuleDialog`・`ModuleSettingsDialog`
は、`<form>` の `onsubmit` と決定のボタンの `onclick` が同じ関数を呼ぶ形のまま残っている (T0953 の候補)。送る道を 1 つにそろえる。

## 受け入れ条件

- [ ] 2 つのダイアログの決定のボタンが `type="submit"` (`form` の口) で `<form>` を送り、`onclick` で同じ関数を呼ばない。Enter と決定のボタンの両方で 1 回だけ送る
- [ ] 部品のテスト (Enter で送る・ボタンで送る・送れないとき (名前が不正・スロットが無い) は送らない・送るのは 1 回だけ)
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0953 のメモ (やらなかったこと・起票の候補)
- apps/editor/src/lib/NewModuleDialog.svelte・ModuleSettingsDialog.svelte、packages/ui (`Button` の `form`)

## メモ

- T0043 で起票した (元の候補: T0953 (`NewModuleDialog`・`ModuleSettingsDialog` の決定のボタン))。
