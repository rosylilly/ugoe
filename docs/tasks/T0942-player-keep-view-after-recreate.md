---
id: T0942
title: "GPU を失って作り直した後に、Player とエディタのビューポートが利用者のズームとパンを保つか (判断が要る)"
status: done
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

- [x] 案と推奨をメモに書き、利用者の判断を得た
- [x] 決めた形で実装し (A なら文書だけ)、偽の wasm の Vitest と、`packages/player/browser-test` の `player-loss-restore` (本物の wasm) でズームとパンが保たれる (または fit に戻る) ことを確かめる
- [x] docs/ui.md#webpuppet-の寿命 に作り直しでのビューの扱いを書いた
- [x] `just player-browser`・`just ci` が通る

## 参考

- T0939 のメモ (起票の候補)
- packages/player/src/player.svelte.ts (`#afterLoad`・`fit`・`view`)、apps/editor/src/lib/viewport/controller.svelte.ts

## メモ

- T0038 で起票した (元の候補: T0939 (作り直しの後の fit))。同じ `Player`・`ViewportController` を触る T0941 の後に着手する。
- 判断 (2026-10-06、利用者の判断): 案 B。作り直しの前のビューを覚えて戻す。モデルを読み直したとき (作り直しでない読み込み) は今までどおり fit。Player とエディタのビューポートで揃える。

### やったこと

- `PuppetSurface` (packages/player/src/surface.ts) が、GPU を失った WebPuppet のビュー (`zoom`・`pan_x`・`pan_y`。パンは CSS ピクセルに
  直す) を `recover`・`restart` で覚え、作り直した WebPuppet に `created(puppet, true)` の前に `set_view` で渡す。続けて失ったときと
  諦めた後の `restart` でも渡す。`attach`・`detach` で忘れる。`SurfacePuppet` に `zoom`・`pan_x`・`pan_y`・`set_view` を足した
  (Player の `wasm.ts` の `WebPuppet` にも `pan_x`・`pan_y`・`set_view`)。
- Player: 作り直しの読み直し (`#setUp` の最初の `#afterLoad`) では fit せず、`zoom`・`view` を WebPuppet から読み直す。作り直しを待つ間
  (WebPuppet が無い) に利用者が開いたモデルと、作り直しの後に開いたモデルは今までどおり fit。
- エディタの `ViewportController`: モデルを読んでもビューを変えない (fit は「全体を表示」と、別のモデルになったときの App.svelte) ので、
  `PuppetSurface` が渡し直すだけで保たれる。コードの変更は無く、テストだけ足した。
- テスト: surface.test.ts (倍率が変わっても CSS ピクセルで同じ場所・次の attach は既定・諦めた後の restart)、player.test.ts (保つ・
  後で開いたモデルは fit・作り直しの間に開いたモデルは fit)、apps/editor の controller.test.ts。偽の WebPuppet (src/testing/fakePuppet.ts)
  にビューの状態を持たせた。browser-test の `player-loss-restore` (2 回の作り直しで `view`・`zoom` が保たれ、開き直すと fit) と
  `player-timeout-restart` (時間切れの作り直しと `restart` の後にビューが戻る)。
- 壊れを見つけることを確かめた: `#restoreView` を呼ばないと surface.test.ts の 2 つ・player.test.ts・`player-loss-restore`・
  `player-timeout-restart` が落ちる。Player の `keepView` を効かせないと / 待つ間に開いたモデルも保つと、player.test.ts の該当テストが落ちる。
- 文書: docs/ui.md#webpuppet-の寿命 (「作り直しでのビュー」と型の受け方)、docs/testing.md (シナリオと壊れの確認)、packages/player/README.md。

### 気づいたこと

- スタジオの出力ウィンドウはシーンの変換を `fit` + `zoomBy` で大きさが変わるたびに入れるが、これまでは作り直しの読み直しの fit で
  次に大きさが変わるまで外れていた。今は保たれる。
- 作り直しを待つ間に別のモデルを開くと、前のモデルの値 (`#carry`) を同じ ID のパラメータに戻す (T0942 の前からの振る舞い。ビューは fit)。

### 確かめていないこと

- 実際の GPU のリセット (WebGPU のデバイスの喪失を含む) と、エディタのビューポートを本物の wasm で (エディタにブラウザのテストは無い。T0939)。
- CI の上での実行 (push していない)。

### 検査

- `git rebase main` の後に `just ci` (前面、終了コード 0) と `just player-browser` (13 のシナリオがすべて ok)。ランタイム層は触っていない。

### 起票の候補

- 作り直しを待つ間に別のモデルを開いたとき、前のモデルのパラメータの値 (`#carry`) を戻さない (ビューと同じく、利用者が開いたモデルは既定から)。
