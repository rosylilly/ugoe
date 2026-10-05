---
id: T0938
title: WebGL2 の復元が届かないときの扱いと、諦めた後に作り直すボタン (エディタ・Player。判断が要る)
status: done
milestone: M9
crates: [packages/player, apps/editor, bindings/web, docs]
depends_on: [T0936]
assignee: claude-worker-T0938
---

# T0938: WebGL2 の復元が届かないときの扱いと、諦めた後に作り直すボタン (エディタ・Player。判断が要る)

## 目的

T0935 で、エディタと Player も WebGL2 のコンテキストの喪失で復元を待ち、続けて失敗したら諦めるようにした。次が残っている (T0935 の候補)。

1. **判断が要る:** 復元 (`webglcontextrestored`) が届かないまま長く待つときの扱い (時間切れで作り直しを試す・「GPU の復元を待っています」と出す)。
   `<ugoe-avatar>` (bindings/web) と同じ扱いにするので一緒に決める。
2. 諦めた後に、起動し直さずに作り直せるボタン (Player・エディタのビューポート。`<ugoe-avatar>` の `restart()` に当たるもの)。

`PuppetSurface` を触る T0936 の後に着手する。

## 受け入れ条件

- [x] 復元を待つ間の表示と時間切れの扱いの案と推奨をメモに書き、利用者の判断を得た。決めた扱いをエディタ・Player・`<ugoe-avatar>` で揃え、docs に書いた
- [x] 諦めた後の「作り直す」ボタンがエディタのビューポートと Player にあり、押すと読み込み直す (モデルと着せ替えを保つ)。部品のテストがある
- [x] 文言は `@ugoe/i18n` のカタログ (日英) にある
- [x] `just ci` が通る

## 参考

- T0935 のメモ (起票の候補)
- docs/ui.md、bindings/web/README.md

## メモ

- T0032 で起票した (元の候補: T0935 (2 つ))。
- 判断 (2026-10-05、利用者の判断): 案 A。喪失したらすぐに「GPU の復元を待っています」を出し、数秒 (例: 5 秒) 復元が届かなければ新しい canvas で作り直しを試す。続けて失敗すれば今の上限 (1 分に 4 回) で諦め、作り直すボタンを出す。エディタ・Player・`<ugoe-avatar>` で揃える。
- やったこと (T0938):
  - `PuppetSurface` (packages/player/src/surface.ts): 喪失に気づいたらすぐ `waiting` で知らせ、`RESTORE_TIMEOUT_MS` (5 秒) 待っても
    `webglcontextrestored` が届かなければ新しい canvas で作り直しを試す。作り直しで作れなければ `waiting` で知らせて 5 秒ごとに試し直し、
    試し直しも作り直しの回数に数えて、今の上限 (1 分に 4 回目) で `failed(GpuLostRepeatedlyError)`。諦めた後の `restart()` (同じ container で
    作り直し、回数を数え直し、`created(puppet, true)`) を足した。
  - Player: `waitingForGpu`・`canRestart`・`restart()` (モデル・モジュール・値を戻す)。ビューアの描く領域 (apps/viewer/src/lib/Viewport.svelte)
    に「GPU の復元を待っています」と「作り直す」ボタン。
  - エディタ: ViewportController の `waiting` で `status` を `loading`・文言 `viewport.gpuWaiting`、`canRestart`・`restart()`、ビューポートの
    右上に「作り直す」ボタン。
  - `<ugoe-avatar>` (MIT。packages/ のコードは写さず、同じ振る舞いを自分で実装): 5 秒の時間切れ・作れないときの試し直し・`gpulost` イベントと
    `waitingForGpu`・待ち始めたら `ready` を新しい Promise に変える。`restart()` は今のまま (試し直さない。作成の失敗のやり直しにも使うため)。
  - 文言: player・viewer・editor のカタログ (日英)。`gpuLostRepeatedly` は「作り直す」を案内する文に変えた。
  - テスト: surface.test.ts・player.test.ts・controller.test.ts・avatar.test.ts、部品のテスト (apps/viewer と apps/editor の
    Viewport.svelte.test.ts)、ブラウザのシナリオ `context-loss-timeout`・`restart-after-giving-up`・`player-timeout-restart`
    (`just player-browser` で 10/10 通った)。run.mjs は Chrome を `--max-active-webgl-contexts=256` で起動する (作り直しを繰り返すと捨てた
    canvas のコンテキストが上限 16 に数えられ、今のコンテキストが失われることがあったため。docs/testing.md に書いた)。
  - docs: docs/ui.md#webpuppet-の寿命・docs/editor.md・docs/testing.md・bindings/web/README.md。
- 範囲外の小さな変更: apps/viewer (Viewport.svelte・カタログ・部品のテスト)。Player の画面の部品はビューアにあるため。
- やらなかったこと:
  - スタジオの出力ウィンドウにはボタンも待つ表示も置いていない (Player の `error` を画面に出していないため)。
  - 本物の GPU のリセット (ドライバの再起動など) では確かめていない (確かめたのは `WEBGL_lose_context` と SwiftShader)。
  - tests/conformance の `<ugoe-avatar>` のシナリオには時間切れを足していない (範囲外。偽の wasm の avatar.test.ts で確かめた)。
- 起票の候補:
  - ブラウザのシナリオで、Chrome がコンテキストを作れないときに ugoe-wasm が glow の中で panic した (`glow-0.17.0/src/web_sys.rs:285` の
    `Option::unwrap()`。コンテキストの上限に当たったとき)。`WebPuppet.create` が例外ではなく panic になるので、作れないことを例外で返すようにする。
  - tests/conformance の `avatar` に、復元が届かないときの作り直しのシナリオを足す。

