---
id: T0932
title: WebPuppet の作成・作り直し・破棄をエディタと @ugoe/player で共有する
status: done
milestone: M9
crates: [ugoe-wasm, packages, bindings/web, apps/editor, docs]
depends_on: [T0914, T0672]
assignee: claude-worker-T0932
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

- [x] まとめ方 (置き場所・型の受け方・エディタの `generation` との関係) を決め、docs/ui.md か ADR に書く
- [x] まとめるなら、エディタと Player がその部品を使い、両方の既存のテストが通る
- [x] `WebPuppet::create` が WebGL2 で作ったとき、canvas の `webglcontextlost` で `lost` を立てる (リスナーは `free` で外す)。`is_lost` の doc コメントと crates/ugoe-wasm/README.md に書く (T0697)
- [x] エディタ・Player・`<ugoe-avatar>` が WebGL2 のコンテキストの喪失から作り直せることを確かめた (テストか、手で確かめた手順をメモに。`<ugoe-avatar>` の本物のブラウザでのテストは T0440)。
      エディタと Player の canvas の `webglcontextlost` のリスナーは、wasm の知らせと重ならないように外すか残す理由を書く
- [x] devicePixelRatio だけの変化 (`matchMedia`) の扱いを、まとめた部品と `<ugoe-avatar>` で揃えた (T0914 の横断)
- [x] `just ci` と `just wasm-check` が通る

## 参考

- apps/editor/src/lib/viewport/controller.svelte.ts (T0672)
- packages/player/src/player.svelte.ts (T0914)
- docs/adr/0003-editor-viewport-in-webview.md
- crates/ugoe-wasm/src/web.rs の `lost`・`is_lost`、まとめる前の T0697

## メモ

### やったこと

- **まとめ方 (docs/ui.md#webpuppet-の寿命):** `@ugoe/player` に `PuppetSurface` (`packages/player/src/surface.ts`) を置き、Player と
  エディタの `ViewportController` が使う。WebPuppet の型は `SurfacePuppet` (`free`・`is_lost`・`resize`) の構造の型で受け、
  `created(puppet, recovered)` にはアプリの型のまま渡す。canvas は `PuppetSurface` が作るので、エディタの `generation` と
  Viewport.svelte の `{#key}` は除いた (Viewport.svelte は canvas を置く要素を渡す)。WebGL2 への切り替えは一度落ちたら作り直しでも
  WebGL2 から (エディタの元の振る舞い)。`<ugoe-avatar>` は MIT なので使わず、違い (復元を待つ・続けて失ったら諦める) を同じ節に書いた。
- **wasm (T0697):** `WebPuppet::create` が WebGL2 (wgpu の `Gl`) で作ったとき、canvas の `webglcontextlost` で `lost` を立てる
  (`Closure` を持ち、`Drop` (= `free`) で外す。既定の動作は止めない)。`is_lost` の doc コメントと crates/ugoe-wasm/README.md、
  bindings/web/README.md に書いた。
- **リスナーを残す理由:** `PuppetSurface` は canvas の `webglcontextlost` も聞いてすぐ作り直す。エディタは要求されたときだけ描くので、
  `is_lost()` だけでは次に描くまで真っ白になるため。作り直しは一度に 1 つ・捨てた WebPuppet と古い canvas の知らせは見ないので、
  wasm の印と重なっても作り直しは 1 回 (surface.test.ts の "recreates once when both …")。`<ugoe-avatar>` のリスナーは
  `preventDefault` して復元を待つために残す (README に書いた)。
- **devicePixelRatio:** `PuppetSurface` が `matchMedia` で聞く (エディタの Viewport.svelte と Player の別々の実装を移した)。
  `<ugoe-avatar>` にも同じ聞き方を足した (ResizeObserver の CSS の大きさ × 新しい倍率。avatar.test.ts)。

### 確かめたこと (WebGL2 のコンテキストの喪失)

- `<ugoe-avatar>`: `just conformance --only avatar` (本物の wasm とヘッドレスの Chrome の WebGL2) の `context-loss` に、`loseContext()`
  の後で `WebPuppet.is_lost()` が真になることを足した (`tests/conformance/web/avatar.js`。wasm の変更を戻すと落ち、戻すと通るのを確かめた)。5 つとも通る。
- `PuppetSurface` (エディタと Player が共有するもの): 本物の wasm (bindings/web/dist) と `tsc` で JS にした surface.ts を
  ヘッドレスの Chrome (`--headless=new`、WebGL2 は SwiftShader) で開くページを手で書き、`WEBGL_lose_context.loseContext()` を 2 回起こした。
  どちらも `lost` の時点で `is_lost()` が真、新しい canvas の WebGL2 で作り直してモデルを読み直し、canvas は 1 つだけ残り、`detach` で 0 になった。
  (手順: canvas の要素を `attach` → `created` で `load_bundle` と `render` → canvas の `getContext("webgl2").getExtension("WEBGL_lose_context").loseContext()`
  → `surface.puppet` が替わるまでフレームを待つ。ページは一時ディレクトリに置いた。)
- エディタと Player: 偽の wasm で、`webglcontextlost` と `is_lost()` のどちらからも作り直すことをテストした (controller.test.ts・
  player の既存のテスト・surface.test.ts)。Tauri のエディタとビューアの実機での喪失は確かめていない (WebKitGTK の WebView で
  `WEBGL_lose_context` を起こす手段を用意していない)。
- 範囲の外の変更: `tests/conformance/web/avatar.js` に `is_lost()` の確かめを 3 行足した (小さいので行った)。

### 検査

- `git rebase main` (T0930・T0933 の後) の後に `just ci` (前面、終了コード 0)・`just wasm-check`・`just conformance --only avatar` (5 つとも ok) が通った。

### やらなかったこと

- エディタと Player は、WebGL2 の喪失で復元 (`webglcontextrestored`) を待たずに新しい canvas で作り直す (T0672・T0914 の振る舞いのまま)。
  本物の GPU のリセットでは、復元の前に新しいコンテキストを作れないことがある (そのときは「描画を始められない」になる)。

### 起票の候補

- エディタと Player でも WebGL2 の喪失で復元を待つか (`<ugoe-avatar>` と揃えるか) を決める。作り直しが続けて失敗したときの諦め方も。
- `PuppetSurface` を本物の wasm とブラウザで確かめる自動のテスト (今回は手で書いたページ。packages/ は GPL なので、MIT の
  tests/conformance に入れるかは置き場所を決める必要がある)。

