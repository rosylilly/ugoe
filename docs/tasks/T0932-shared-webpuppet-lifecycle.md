---
id: T0932
title: WebPuppet の作成・作り直し・破棄をエディタと @ugoe/player で共有する
status: todo
milestone: M9
crates: [ugoe-wasm, packages, bindings/web, apps/editor, docs]
depends_on: [T0914, T0672]
assignee:
---

# T0932: WebPuppet の作成・作り直し・破棄をエディタと @ugoe/player で共有する

## 目的

T0914 のレビューで見つけた。エディタの `apps/editor/src/lib/viewport/controller.svelte.ts` と
`packages/player/src/player.svelte.ts` が、同じ仕組みをそれぞれ持っている:

- WebGPU で作れなければ WebGL2 に落とす (1 つの canvas は併用できない)
- GPU を失ったら作り直す (WebGPU は `is_lost`、WebGL2 は `webglcontextlost`。T0672・T0914 で両方に別々に足した)
- 作っている間に detach や次の attach があったら、遅れてできた WebPuppet を捨てる (T0672・T0914 で両方に別々に直した)
- devicePixelRatio の変化に合わせて描画バッファの大きさを直す (T0672 はエディタの Viewport.svelte、T0914 は Player)

同じ誤りが両方に入り、片方だけ直る形になっている。WebPuppet の寿命 (作る・作り直す・捨てる) を 1 つの部品にまとめて
両方から使うか、まとめない理由を決めて書く。wasm の口 (`WebPuppet` の型) はアプリごとに違うので、構造の型で受ける。

WebGL2 のコンテキストの喪失を wasm の側で知らせる T0697 (T0672 のレビュー) をこのタスクにまとめた (T0008)。`WebPuppet.is_lost()` は wgpu のデバイスの
喪失しか見ず、wgpu-hal の GL は `webglcontextlost` を知らせない。エディタは T0672、Player は T0914 で canvas の `webglcontextlost` を聞くようにしたが、
`<ugoe-avatar>` (`bindings/web/src/avatar.ts`) は MIT で GPL の `packages/` を使えないので、喪失の知らせは wasm の側 (`is_lost`) に寄せるのがよい。
寿命の部品をまとめるのと同じコードを触るので一緒に決める。

## 受け入れ条件

- [ ] まとめ方 (置き場所・型の受け方・エディタの `generation` との関係) を決め、docs/ui.md か ADR に書く
- [ ] まとめるなら、エディタと Player がその部品を使い、両方の既存のテストが通る
- [ ] `WebPuppet::create` が WebGL2 で作ったとき、canvas の `webglcontextlost` で `lost` を立てる (リスナーは `free` で外す)。`is_lost` の doc コメントと crates/ugoe-wasm/README.md に書く (T0697)
- [ ] エディタ・Player・`<ugoe-avatar>` が WebGL2 のコンテキストの喪失から作り直せることを確かめた (テストか、手で確かめた手順をメモに。`<ugoe-avatar>` の本物のブラウザでのテストは T0440)。
      エディタと Player の canvas の `webglcontextlost` のリスナーは、wasm の知らせと重ならないように外すか残す理由を書く
- [ ] devicePixelRatio だけの変化 (`matchMedia`) の扱いを、まとめた部品と `<ugoe-avatar>` で揃えた (T0914 の横断)
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- apps/editor/src/lib/viewport/controller.svelte.ts (T0672)
- packages/player/src/player.svelte.ts (T0914)
- docs/adr/0003-editor-viewport-in-webview.md
- crates/ugoe-wasm/src/web.rs の `lost`・`is_lost`、まとめる前の T0697

## メモ
