---
id: T0479
title: "適合テスト: <ugoe-avatar> で復元が届かないときの作り直しのシナリオ"
status: done
milestone: M9
crates: [tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0479
---

# T0479: 適合テスト: <ugoe-avatar> で復元が届かないときの作り直しのシナリオ

## 目的

T0938 で、`<ugoe-avatar>` は WebGL2 のコンテキストを失って 5 秒のうちに復元が届かなければ新しい canvas で作り直す (作れなければ試し直し、
上限で諦める) ようになった。これは偽の wasm の `avatar.test.ts` でだけ確かめていて、本物の wasm と Chrome で動かす
tests/conformance の `avatar` のシナリオには時間切れが無い (T0938 の候補)。

## 受け入れ条件

- [x] tests/conformance の `avatar` に、`WEBGL_lose_context` で失わせて復元させないシナリオを足し、`gpulost` (`waitingForGpu`)・時間切れの後の作り直し・
      作り直した後に描けること (画素か `ready`) を確かめる
- [x] 作り直しを続けて失敗させたときに上限で諦めること (`error`) も確かめるか、確かめないなら理由をメモに書いた
- [x] 時間切れの待ちで適合テストが大きく遅くならない (かかった時間をメモに書く)
- [x] tests/conformance/README.md を直した
- [x] `just conformance`・`just ci` が通る

## 参考

- T0938 のメモ (やらなかったこと・起票の候補)
- bindings/web/src/avatar.ts、tests/conformance/README.md、packages/player/browser-test (PuppetSurface の同じシナリオ `context-loss-timeout`)

## メモ

- T0038 で起票した (元の候補: T0938 (適合テストの avatar))。
- やったこと:
  - tests/conformance/web/avatar.js に 2 つのシナリオを足した。
    - `context-loss-timeout`: `loseContext()` して復元させない。`gpulost` を 1 回送り `waitingForGpu` が真、`ready` が新しい Promise に
      変わる。2.5 秒の時点ではまだ作り直していない。時間切れ (5 秒) の後に新しい canvas で作り直し (`load`)、`ready` がその `WebPuppet` で
      解決し、モデルとパラメータの値が戻って描く (画素)。喪失から作り直しまでを測って 5 秒 - 100 ミリ秒以上であることも確かめる。
    - `gpu-lost-repeatedly`: 要素が作る新しい canvas のコンテキストを `WebPuppet.create` の前に `WEBGL_lose_context` で失わせ、本物の wasm の
      `CreateError` (`context_unavailable`。T0475) で作り直しを失敗させる。1 回目の喪失は作り直し (1 回目) で作れず、`error` を送らずに待ち、
      5 秒後の試し直し (2 回目) で作って描く。2 回目の喪失は作り直し (3 回目 = `MAX_RESTARTS`) で作れず、5 秒後の試し直しで上限を超えて
      諦め、`error` を 1 回送り、待っていた `ready` をその Error で reject し、`WebPuppet` も canvas も残らない。`restart()` で作り直して
      モデルと値が戻って描く。
  - tests/conformance/run.mjs: Chrome を `--max-active-webgl-contexts=256` で起動する (新しいシナリオを足すと「Too many active WebGL
    contexts. Oldest context will be lost.」が後のシナリオで 5 回出た。今のコンテキストが失われると不安定になるので、
    packages/player/browser-test と同じにした。足した後は 0 回)。
  - tests/conformance/README.md (シナリオの表・作れないことの起こし方・かかる時間・Chrome の起動の設定)、run.mjs と avatar.js の先頭の説明。
  - 範囲外の小さな変更: bindings/web/README.md の検査の表 (`avatar` の説明に時間切れ・試し直し・諦めを足した 1 行)。
- かかった時間 (このマシン、ほかのワーカーと並行で load average 20〜30): `context-loss-timeout` 5.5〜5.9 秒、`gpu-lost-repeatedly` 11.0〜11.2 秒。
  `just conformance --only avatar --no-web-build` は足す前 12 秒、足した後 29〜39 秒。`just conformance` 全体は 3 分 29 秒なので、
  増えたのは 1 割ほど (時間切れは本物の 5 秒を待つ。時計を差し替えて縮めることはしなかった。本物の時間で要素の定数を確かめるため)。
- 気づいたこと: `restoreContext()` を `gpulost` の Promise の続きで同期的に呼ぶと、Chrome は「context restoration not allowed」で
  復元しない (`gpulost` は `webglcontextlost` の処理の中で送られ、`preventDefault` がまだ反映されていない)。シナリオは 2 フレーム待ってから戻す。
  最初の版はこれで復元が届かず、時間切れの経路で偶然通っていた (ログの待ち時間が 10 秒だったので気づいた)。
- テストが壊れを見つけること: dist/avatar.js の `RESTORE_TIMEOUT_MS` を大きくすると 2 つとも、`MAX_RESTARTS` を 4 にすると
  `gpu-lost-repeatedly` が落ちることを確かめた (元に戻した)。
- 確かめたこと: `just conformance` (c・wasm・wasm-webgpu・avatar・queries・unity・godot すべて ok)・`just ci`。
- 確かめていないこと: 本物の GPU のリセットで復元が届かない場面 (SwiftShader と `WEBGL_lose_context` だけ)。
- 起票の候補:
  - `<ugoe-avatar>` で作り直しが `CreateError` で失敗すると、`#start` の catch と、失わせた新しい canvas の `webglcontextlost` の処理が、
    それぞれ `#restoreTimer` に時間切れを入れる (前の値を消さずに上書きする)。今は、先に来た方が `#onLost` → `#stop` で `#session` を
    進めるか `#restoreTimer` を消すので二重には数えないが、片方が残ったまま走る。上書きの前に `clearTimeout` する (PuppetSurface も同じか確かめる)。
