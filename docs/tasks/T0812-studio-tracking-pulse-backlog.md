---
id: T0812
title: スタジオ: 刻みで回すトラッキングが溜まって固まる不具合を直す
status: in-progress
milestone: M8
crates: [apps/studio, packages/face-tracking, docs]
depends_on: [T0803]
assignee: claude-worker-T0812
---

# T0812: 刻みの溜まりでトラッキングが止まる

## 目的

利用者の報告: スタジオでカメラのトラッキングがまったく動かない。調査の結果 (実機の Tauri の WebKitGTK で再現):

- スタジオはカメラとマイクの処理をバックエンドの刻み (`subscribe_pulse`、60/s) で回すが、処理が追いつかないときに刻みを間引かない
  (`apps/studio/src/lib/pulse.ts` の `beat`)。MediaPipe の検出 1 回が刻みの間隔より重いと (WSL2 では顔と体で約 200ms)、WebView に刻みのメッセージが際限なく溜まり、
  `set_tracking` の応答が十数秒待たされて、指揮役に値がほとんど届かない。ビューアは `requestAnimationFrame` で回すので溜まらない。
- 別に、`@tauri-apps/api` の `Channel` は `onmessage` が一度でも例外を投げると、それ以降のメッセージを配らなくなる。スタジオの刻み・モニター・出力のフレームの受け取りが
  これに当たる。

## 受け入れ条件

- [ ] `Pulse.beat` は、前の処理が終わってから前の処理にかかった時間が経つまで刻みを飛ばす (処理が重ければ間引く)。1 つの処理が投げても、ほかの処理と次の刻みは動く
- [ ] Channel の `onmessage` で動くもの (刻み・`Control.svelte` のモニター・`Output.svelte` のフレームの受け取り、ほかにあれば全部) は例外を外へ出さない (握りつぶさず console に出す)
- [ ] `@ugoe/face-tracking` の平滑化の dt を、前の刻みからではなく前の検出から測る (ビューア・エディタの動きを変えないことを確かめる)
- [ ] テスト: 処理が刻みより重いと間引かれる、1 つの処理が投げても他は動き次の刻みも動く、Channel の受け取りで投げても次のメッセージが処理される
- [ ] docs/studio.md の「隠れても止まらない」に、刻みで回す処理は重ければ間引くことを書く
- [ ] 重い検出を模した入力 (検出に 200ms かかる偽の tracker など) で、`set_tracking` が溜まらず追従することを確かめる。`just ci` が通る

## 参考

- 調査の報告 (team-lead の会話。実機の Tauri で測った表と直し方の案)

## メモ
