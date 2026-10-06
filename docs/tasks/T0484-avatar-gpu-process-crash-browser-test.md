---
id: T0484
title: "適合テスト: <ugoe-avatar> で GPU プロセスが落ちても作り直して描く"
status: done
milestone: M9
crates: [tests/conformance, bindings/web, docs]
depends_on: []
assignee: claude-worker-T0484
---

# T0484: 適合テスト: <ugoe-avatar> で GPU プロセスが落ちても作り直して描く

## 目的

T0944 で、GPU プロセスを落とすブラウザのテスト (CDP の `Browser.crashGpuProcess`) を `@ugoe/player` (`just player-browser`) に足し、作成の途中で落ちたときの
glow と wgpu の panic を直した。`<ugoe-avatar>` (bindings/web) の側には同じシナリオが無い (T0944 の候補)。wasm は同じなので panic は直っているが、
`<ugoe-avatar>` の復元の待ちと作り直しが GPU プロセスの喪失でも働くことを確かめる。

## 受け入れ条件

- [x] 適合テストの `avatar` (tests/conformance/run.mjs) に、描いている間に GPU プロセスを落とすと `<ugoe-avatar>` が作り直して描くシナリオと、
      作成の途中に落としても panic せずコードを持つ例外 (と作り直し) で終わるシナリオがある (起動の引数と落とし方は player-browser と揃える)
- [x] 壊れを見つけることを一度確かめた (例: 作り直しを止めるとシナリオが落ちる) とメモに書いた
- [x] docs/testing.md と tests/conformance/README.md を直した
- [x] `just conformance --only avatar` と `just ci` が通る

## 参考

- T0944 のメモ (起票の候補)、docs/testing.md (GPU プロセスの落とし方)
- tests/conformance/run.mjs、bindings/web/src/avatar.ts

## メモ

- T0040 で起票した (元の候補: T0944 (`<ugoe-avatar>` のブラウザのテスト))。
- やったこと:
  - tests/conformance/web/avatar.js にシナリオを 2 つ足した (GPU プロセスを落とすので最後に動かす):
    - `gpu-process-crash`: 描いている間に落とすと、`gpulost` を 1 回送って復元を待ち、新しい canvas で作り直して (`load`)、`ready` が新しい
      `WebPuppet` で解決し、モデルと `setParameter` の値が戻って描く。`error` は送らない。2 回続けて (落としてから作り直すまで 2〜3 秒)。
    - `gpu-process-crash-during-create`: 要素を付けるのと同時に 0〜500 ミリ秒後に落とす (8 回)。wasm は panic せず、要素は作り直して描くか、
      コードを持つ例外 (`CreateError`・`LoadError`・`RenderError`) の `error` を送って止まり、止まったら `restart()` で作り直して描く。
      どう終わったかをログに書く (手元の 2 回の走りは同じ内訳: `CreateError`/`context_unavailable` で止まって `restart()` x1、
      読み込みの途中に失って `LoadError`/`device_lost` を送った後に作り直し x1、作り直し x6)。
    - どのシナリオも、wasm の panic (`console.error` の `panicked at`) があれば失敗にする (`until` も panic ですぐ失敗する)。
  - tests/conformance/run.mjs: avatar のときだけ Chrome を `--disable-gpu-process-crash-limit`・`--disable-domain-blocking-for-3d-apis`
    (player-browser と同じ値) と `--remote-debugging-port=0` で起動し、ページの `POST /crash-gpu` (`{"delay": ミリ秒}` は待たずに応答して後で落とす)
    で CDP の `Browser.crashGpuProcess` を送る。CDP はプロファイルの `DevToolsActivePort` からポートを読み、Node の組み込みの `WebSocket` で
    繋ぐ (playwright は使わない。GPL の packages/player のコードは写さず、起動の設定の値だけ揃えた)。
  - 文書: tests/conformance/README.md (シナリオの表・落とし方・panic)・docs/testing.md#配布物の適合テスト。
- 壊れを見つけること: bindings/web/src/avatar.ts の `#onLost` を何もしないようにすると、`gpu-process-crash` (round 1 の作り直しの時間切れ) と
  `gpu-process-crash-during-create` (delay 20 で落ち着かない) が落ちた (既存の `context-loss` など 3 つも落ちる)。戻して全て通ることを確かめた。
  panic の検出は、panic を起こす wasm を作って確かめてはいない (T0944 の player-browser の検出と同じ仕組み)。
- 確かめたこと: `just conformance --only avatar` (9 シナリオ)・`just conformance` (c・wasm・wasm-webgpu・avatar・queries・unity・godot すべて ok。
  main に rebase した後)・`just ci`。
- 確かめていないこと: 本物の GPU での GPU プロセスの喪失・Chrome 以外のブラウザ。`gpu-process-crash-during-create` は時機が決まらないので、
  全ての段で落としたとは言えない (決定的な確かめは player-browser の `lose-context-during-create` など)。
- 起票の候補:
  - `<ugoe-avatar>` が読み込み (`load_bundle`) の途中に GPU を失うと、`LoadError` (`device_lost`) の `error` を送った後に作り直して読み直す
    (上の内訳の 1 つ)。利用者には作り直せたのに `error` が届く。読み込み・`#start` のモデルの適用で `device_lost` を受けたら、`error` を
    送らずに喪失として待つ (または `webglcontextlost` を待ってから判断する) のがよいか決めて直す (bindings/web、avatar.test.ts に偽物の順序のテスト)。
