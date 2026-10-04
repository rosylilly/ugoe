---
id: T0440
title: npm: <ugoe-avatar> を本物のブラウザと wasm で確かめるテスト (コンテキストの喪失・切り離し)
status: done
milestone: M9
crates: [bindings/web, tests/conformance, docs]
depends_on: [T0425]
assignee: claude-worker-T0440
---

# T0440: <ugoe-avatar> のブラウザのテスト

## 目的

T0425 (bindings/web のレビュー) で、`<ugoe-avatar>` の作成・切り離し・再接続・WebGL2 のコンテキストの喪失からの作り直しを
直し、happy-dom と `WebPuppet` の偽物で順序をテストした (`bindings/web/src/avatar.test.ts`)。本物の wasm と GPU で通るかは
確かめていない (配布物の適合テスト `tests/conformance` は `WebPuppet` だけを使い、要素は使わない)。範囲 (tests/) の外で、
ヘッドレスの Chrome の準備が要るので起票した。

## 受け入れ条件

- [x] ヘッドレスの Chrome で `examples/basic` か専用のページを開き、`<ugoe-avatar>` が `load` を送って描くことを確かめる
- [x] WebGL2 (`backend="webgl2"`) で、shadow root の canvas の `getContext("webgl2").getExtension("WEBGL_lose_context")` の
      `loseContext()`・`restoreContext()` でコンテキストを失わせ、新しい canvas で作り直してモデルとパラメータの値が戻ることを確かめる
- [x] 取得中・作成中の切り離しと再接続、`backend` の変更で、GPU の資源 (`WebPuppet`) が残らず、最後のモデルを描くことを確かめる
- [x] CI のどのジョブで流すかを決める (`just conformance` に足すか、別のジョブか)
- [x] `just ci` が通る

## 参考

- bindings/web/src/avatar.ts・avatar.test.ts、tests/conformance/README.md、tests/conformance/web/runner.js
- T0697 (ugoe-wasm が WebGL2 のコンテキストの喪失を知らせる口。T0672 で起票)

## メモ

- **やったこと:** `tests/conformance/run.mjs` にバインディング `avatar` を足した。サーバーが専用のページ
  `tests/conformance/web/avatar.html`・`avatar.js` を配り、ヘッドレスの Chrome (WebGL2、SwiftShader) で本物の wasm
  (`bindings/web/dist`) の `<ugoe-avatar>` を 5 つのシナリオ (`load-and-draw`・`context-loss`・`detach-while-creating`・
  `detach-while-fetching`・`backend-change`) で動かす。結果は `target/conformance/avatar/avatar.json`。中身は
  tests/conformance/README.md の「`<ugoe-avatar>` のシナリオ」。
  - GPU の資源: `WebPuppet.create`・`free` を包んで生きている数を数え、作成中のものが終わってから数を確かめる。
    各シナリオの後で要素を外し、0 に戻らなければそのシナリオの失敗にする。
  - 描いたこと: shadow root の canvas を 2D の canvas に写し、背景でない画素が 1% を超えるか。
  - 取得中の操作のため、サーバーの `/samples/<モデル>?delay=ミリ秒` で応答を遅らせる。
  - モデルの見分けは描画するメッシュの ID の並び (要素を使わずに `WebPuppet` で読んで求める)。
- **CI のジョブ:** `just conformance` (CI の `conformance` ジョブ) に含めた。同じ Chrome とパッケージのビルドを使い、
  増える時間は 30 秒ほど。`--only avatar` なら参照を作らないので cargo のビルドも要らない。`ci.yml` は変えていない。
- **確かめたこと (WSL2、Chrome 154):** 5 つのシナリオが 5 回とも通った。`--only c,wasm,avatar --cases avatar-pose,masks-moving`
  で既存の wasm と C ABI も通った (run.mjs の書き換えの確認)。テストが誤りを見つけることを、`dist/avatar.js` を壊して確かめた
  (コミットしていない): `webglcontextlost` の `preventDefault` を消す → `context-loss` が失敗、作成中に切った `WebPuppet` を
  `free` しない → `detach-while-creating`・`backend-change` が失敗、切断で取得を捨てる (T0425 の前の挙動) →
  `detach-while-fetching`・`backend-change` が失敗、`render()` を呼ばない → すべて失敗。
- **見つかったこと:** コンテキストを失った後に古い `WebPuppet` を `free` すると、Chrome のコンソールに
  `WebGL: INVALID_OPERATION: delete: object does not belong to this context` が数十行出る (wgpu-hal の GL が失ったコンテキストの
  資源を消す)。害は無い (作り直しは通る) ので、そのままにした。
- **範囲の外の変更 (小さなもの):** `justfile` の `conformance` のコメント (avatar を足し、消えた `--backend webgpu` の例を
  `--only wasm-webgpu` に直した)、`docs/testing.md` の適合テストの説明に `<ugoe-avatar>` を足した。
- **やらなかったこと:** WebGPU のデバイスの喪失 (`is_lost`) はページから起こせないので確かめていない (偽物のテストだけ)。
  CI の上では動かしていない (push しないため)。Chrome 以外のブラウザ (Firefox・Safari) では確かめていない。
- **起票の候補:** なし (WebGPU の喪失を起こす口が要るなら、T0697 で ugoe-wasm に知らせる口を作るときに一緒に考える)。
