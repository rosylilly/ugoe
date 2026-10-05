---
id: T0942
title: "GPU を失って作り直した後に、Player とエディタのビューポートが利用者のズームとパンを保つか (判断が要る)"
status: in-progress
milestone: M9
crates: [packages/player, apps/editor, docs]
depends_on: [T0941]
assignee: claude-worker-T0942
---

# T0942: GPU を失って作り直した後に、Player とエディタのビューポートが利用者のズームとパンを保つか (判断が要る)

## 目的

WebGL2 のコンテキストを失って作り直すと、`Player` は `#setUp` → `load` → `#afterLoad` で `fit()` を呼び、利用者が動かしたズームとパンが
fit に戻る (T0939 の候補)。モデル・モジュール・パラメータの値は戻すのに、ビューだけが戻らない。

**判断が要る:** 案:

- A. 今のまま (作り直しは fit に戻す)。
- B. 作り直しの前のビュー (`view_transform`) を覚えて戻す。モデルを読み直したとき (作り直しでない読み込み) は今までどおり fit。**推奨**
  (作り直しは利用者の操作ではないので、見ている場所を変えない)。

エディタのビューポート (`ViewportController`) が作り直しでビューを保つかも確かめ、同じ規則に揃える。

## 受け入れ条件

- [ ] 案と推奨をメモに書き、利用者の判断を得た
- [ ] 決めた形で実装し (A なら文書だけ)、偽の wasm の Vitest と、`packages/player/browser-test` の `player-loss-restore` (本物の wasm) でズームとパンが保たれる (または fit に戻る) ことを確かめる
- [ ] docs/ui.md#webpuppet-の寿命 に作り直しでのビューの扱いを書いた
- [ ] `just player-browser`・`just ci` が通る

## 参考

- T0939 のメモ (起票の候補)
- packages/player/src/player.svelte.ts (`#afterLoad`・`fit`・`view`)、apps/editor/src/lib/viewport/controller.svelte.ts

## メモ

- T0038 で起票した (元の候補: T0939 (作り直しの後の fit))。同じ `Player`・`ViewportController` を触る T0941 の後に着手する。
- 判断 (2026-10-06、利用者の判断): 案 B。作り直しの前のビューを覚えて戻す。モデルを読み直したとき (作り直しでない読み込み) は今までどおり fit。Player とエディタのビューポートで揃える。
