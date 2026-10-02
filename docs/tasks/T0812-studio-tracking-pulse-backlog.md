---
id: T0812
title: スタジオ: 刻みで回すトラッキングが溜まって固まる不具合を直す
status: done
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

- [x] `Pulse.beat` は、前の処理が終わってから前の処理にかかった時間が経つまで刻みを飛ばす (処理が重ければ間引く)。1 つの処理が投げても、ほかの処理と次の刻みは動く
- [x] Channel の `onmessage` で動くもの (刻み・`Control.svelte` のモニター・`Output.svelte` のフレームの受け取り、ほかにあれば全部) は例外を外へ出さない (握りつぶさず console に出す)
- [x] `@ugoe/face-tracking` の平滑化の dt を、前の刻みからではなく前の検出から測る (ビューア・エディタの動きを変えないことを確かめる)
- [x] テスト: 処理が刻みより重いと間引かれる、1 つの処理が投げても他は動き次の刻みも動く、Channel の受け取りで投げても次のメッセージが処理される
- [x] docs/studio.md の「隠れても止まらない」に、刻みで回す処理は重ければ間引くことを書く
- [x] 重い検出を模した入力 (検出に 200ms かかる偽の tracker など) で、`set_tracking` が溜まらず追従することを確かめる。`just ci` が通る

## 参考

- 調査の報告 (team-lead の会話。実機の Tauri で測った表と直し方の案)

## メモ

- `Pulse.beat` (apps/studio/src/lib/pulse.ts) は、前の処理が終わってから前の処理にかかった時間が経つまで刻みを飛ばす。各処理は try/catch で包み、
  投げたものは console.error に出す。時計は差し替えられる (テスト用)。
- Channel の受け取り: スタジオの Tauri の Channel はすべて host.ts の `subscribe` を通るので、そこで `safeChannelHandler` (lib/channel.ts) に包んだ
  (刻み・`subscribeStatus` (onMonitor)・`subscribeFrames` (Output.svelte))。テストは本物の `@tauri-apps/api` の `Channel` に偽の
  `transformCallback` を入れ、包まないと止まること・包むと次のメッセージが届くことを確かめる。ブラウザソースの WebSocket は投げても止まらないので触っていない。
- 平滑化の dt: `@ugoe/face-tracking` に `FrameInterval` を足し、`FaceTracker` は値が出た刻み (検出・再生) の間で dt を測る (上限は従来の MAX_DT = 0.1 秒のまま)。
  毎刻み値が出るとき (再生・刻みと同じ速さの入力) はこれまでと同じ dt になることをテストで確かめた。60Hz の requestAnimationFrame で 30fps のカメラを
  使うビューア・エディタでは、dt が 1/60 から実際の 1/30 になる (平滑化が実時間に合う方向の変化)。ビューア・エディタを実物で動かしての確認はしていない。
- 実機の Tauri (WSL2 の WebKitGTK、debug ビルド) で確かめた: 調査役の probe (scratchpad/debug-tracking/e2e を写したもの。偽のカメラ = 回転する顔の
  canvas の captureStream、MediaPipe の顔+体の検出は 1 回約 200ms) で、カメラを始めてから 25 秒間、set_tracking が約 3 回/秒 (25 秒で 60 回)、応答の p50 が
  17〜22ms (まれに 46ms)、指揮役の AngleZ がカメラのロール (±18 度) に追従し、出力ウィンドウは 60 フレーム/秒で受け取り続けた。
  直す前 (調査役の記録) は 50 秒で set_tracking 3 回、タイマーの遅れ 17 秒。
- 確かめていないこと: Windows (WebView2)・macOS (WKWebView) の実機、実際のカメラ。

