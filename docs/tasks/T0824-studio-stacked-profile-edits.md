---
id: T0824
title: スタジオ: 続けて変えたプロファイルの設定を、通知を待たずに積み重ねる
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0815]
assignee:
---

# T0824: スタジオ: 続けて変えたプロファイルの設定を、通知を待たずに積み重ねる

## 目的

操作ウィンドウのパネル (ホットキー・表情とモーション・入力) は、変えるたびに「今の props の写し + 変更」をバックエンドへ送り、
画面の値は `profile` の通知が届いてから変わる。通知が届く前に続けて変えると、2 つ目の変更が古い props を基にするので、
1 つ目の変更が消える (T0815 のレビューで見つけた)。

- `HotkeysPanel.svelte` の `update`・`add`・`remove` (割り当てを足した直後にキーを押すと、足した行がまだ props に無く、押したキーが捨てられる)
- `MotionPanel.svelte` の `change` (トラッキングのままにするまとまりを続けて切り替える)
- `InputPanel.svelte` の `change` (マイクの機器と感度)

シーンの編集は `Control.svelte` の `editing` (送っている途中の編集を先に画面に出す) で避けている。同じ考え方を、プロファイルの
ほかの設定にも当てる (送っている途中の値を基にして次の変更を作り、通知が追いついたら通知の値に戻す)。

## 受け入れ条件

- [ ] 通知が届く前に続けて変えても、どの変更も消えない (ホットキー・モーションの設定・入力の設定)
- [ ] 割り当てを足した直後に押したキーが、その割り当てに入る
- [ ] 振る舞いのテストがある (部品のテストは happy-dom。apps/studio/vitest.config.ts)
- [ ] `just ci` が通る

## 参考

- docs/studio.md#操作ウィンドウ
- apps/studio/src/Control.svelte の `updateScene` と `editing`

## メモ
