---
id: T0824
title: スタジオ: 続けて変えたプロファイルの設定を、通知を待たずに積み重ねる
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0815]
assignee: claude-worker-T0824
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

- [x] 通知が届く前に続けて変えても、どの変更も消えない (ホットキー・モーションの設定・入力の設定)
- [x] 割り当てを足した直後に押したキーが、その割り当てに入る
- [x] 振る舞いのテストがある (部品のテストは happy-dom。apps/studio/vitest.config.ts)
- [x] `just ci` が通る

## 参考

- docs/studio.md#操作ウィンドウ
- apps/studio/src/Control.svelte の `updateScene` と `editing`

## メモ

- やったこと:
  - `apps/studio/src/lib/pendingEdit.svelte.ts` の `PendingEdit` を足した。送っている途中の値を持ち (`current` は途中の値、無ければ届いた値)、
    送るのは 1 つずつ、送っている間の変更は最後のものだけを次に送り、送り終えたら (失敗しても) 途中の値を捨てて届いた値に戻す。
    `Control.svelte` の `updateScene` と `editing` と同じ考え方。バックエンドの `with_stage` はコマンドを返す前に `profile` の通知を配るので、
    送り終えた時点で通知は届いている。
  - `HotkeysPanel`・`MotionPanel`・`InputPanel` は props の値ではなく `current` (送っている途中の値を含む) を基にして次の変更を作り、画面にも出す。
    コールバック (`onchange`・`onsettings`) は Promise を返す形にし、`Control.svelte` は `run(...)` の Promise をそのまま返す。
    `InputPanel` のトラッキングの調整の書き戻し (`rememberTrackerSettings`) も同じ `PendingEdit` を通す (マイクの変更と積み重なる)。
    割り当てを足した行は、通知を待たずに出る (足した直後に押したキーがその行に入る)。
  - テスト (happy-dom): `lib/pendingEdit.svelte.test.ts`・`HotkeysPanel.svelte.test.ts`・`MotionPanel.svelte.test.ts`・`InputPanel.svelte.test.ts`。
    部品のテストは、修正前の部品では失敗することを確かめた。
  - docs/studio.md#操作ウィンドウ に、送っている途中の値の扱いを書いた。
- やらなかったこと:
  - マイクの感度と閾値を `Microphone` に当てるのは、今までどおり届いた値 (`Control.svelte` の `inputSettings`) から。送っている間 (通知までの一瞬) は前の値で動く。
  - e2e (`just e2e --only studio`) は流していない (スタジオの e2e はこれらのパネルを操作しない)。
- 起票の候補: なし
