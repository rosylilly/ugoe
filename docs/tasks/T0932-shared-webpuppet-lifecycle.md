---
id: T0932
title: WebPuppet の作成・作り直し・破棄をエディタと @ugoe/player で共有する
status: todo
milestone: M9
crates: [packages, apps/editor, docs]
depends_on: [T0914]
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

## 受け入れ条件

- [ ] まとめ方 (置き場所・型の受け方・エディタの `generation` との関係) を決め、docs/ui.md か ADR に書く
- [ ] まとめるなら、エディタと Player がその部品を使い、両方の既存のテストが通る
- [ ] `just ci` が通る

## 参考

- apps/editor/src/lib/viewport/controller.svelte.ts (T0672)
- packages/player/src/player.svelte.ts (T0914)
- docs/adr/0003-editor-viewport-in-webview.md

## メモ
