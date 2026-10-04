---
id: T0440
title: npm: <ugoe-avatar> を本物のブラウザと wasm で確かめるテスト (コンテキストの喪失・切り離し)
status: in-progress
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

- [ ] ヘッドレスの Chrome で `examples/basic` か専用のページを開き、`<ugoe-avatar>` が `load` を送って描くことを確かめる
- [ ] WebGL2 (`backend="webgl2"`) で、shadow root の canvas の `getContext("webgl2").getExtension("WEBGL_lose_context")` の
      `loseContext()`・`restoreContext()` でコンテキストを失わせ、新しい canvas で作り直してモデルとパラメータの値が戻ることを確かめる
- [ ] 取得中・作成中の切り離しと再接続、`backend` の変更で、GPU の資源 (`WebPuppet`) が残らず、最後のモデルを描くことを確かめる
- [ ] CI のどのジョブで流すかを決める (`just conformance` に足すか、別のジョブか)
- [ ] `just ci` が通る

## 参考

- bindings/web/src/avatar.ts・avatar.test.ts、tests/conformance/README.md、tests/conformance/web/runner.js
- T0697 (ugoe-wasm が WebGL2 のコンテキストの喪失を知らせる口。T0672 で起票)

## メモ
