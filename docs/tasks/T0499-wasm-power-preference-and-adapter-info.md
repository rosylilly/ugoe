---
id: T0499
title: "ugoe-wasm: WebPuppet.create で GPU の電力の希望 (powerPreference) を渡せるようにし、使っているアダプタの名前を返す"
status: todo
milestone: M8
crates: [ugoe-wasm, packages/player, docs]
depends_on: []
assignee:
---

# T0499: ugoe-wasm: powerPreference とアダプタの情報

## 目的

利用者の要望「スタジオで使う GPU を選ぶ (外付けの高性能か内蔵の省電力か)」(T0076 で調べた) の、WebView の中の描画の側の下地。
スタジオの出力ウィンドウ (と、エディタ・ビューアのビューポート) は `WebPuppet.create(canvas, backend)` で作るが、今はアダプタを
求めるときに電力の希望を渡していない (`crates/ugoe-wasm/src/web.rs` の `create_with` が `RequestAdapterOptions` を既定のまま渡す)。
また、どの GPU で描いているかを利用者に見せる口が無い (`backend()` は API の名前 (`webgpu`・`webgl2`) だけ)。

- **WebGPU:** wgpu は `PowerPreference` を `navigator.gpu.requestAdapter({ powerPreference })` に渡す (wgpu 30 の backend/webgpu.rs)。
- **WebGL2:** wgpu-hal の GL のバックエンドは `canvas.getContext("webgl2", { antialias: false })` で作り、`powerPreference` を渡さない
  (wgpu-hal 30 の gles/web.rs)。WebGL の仕様では、同じ canvas の 2 回目からの `getContext` は最初のコンテキストを返し属性を無視するので、
  ugoe-wasm が `create_surface` の前に `getContext("webgl2", { antialias: false, powerPreference })` で先にコンテキストを作れば、
  wgpu はそれを使う (依存の変更も unsafe も要らない)。

どちらも **希望 (hint)** で、効くかはブラウザと OS が決める (T0076 のメモ: Windows の Chromium (WebView2) は無視する、2 つの GPU を持つ
macOS で効く、など)。効いたかを確かめられるように、使っているアダプタの名前を返す。

## 受け入れ条件

- [ ] `WebPuppet.create(canvas, backend, powerPreference?)` を足した。`powerPreference` は `"default"`・`"low-power"`・`"high-performance"`
      (省くと `"default"`。ほかの文字列は例外)。WebGPU では wgpu の `PowerPreference` に、WebGL2 では先に作るコンテキストの属性に渡す。
      `antialias: false` は今と同じ (wgpu-hal と同じ属性で作る)
- [ ] WebGL2 で先にコンテキストを作っても、作成の途中の喪失の扱い (T0944 の `LostContextGuard`)・`webglcontextlost` の印・作れなかったときの
      `CreateError` のコードが今と同じであることを、既存のブラウザのテスト (`wasm-pack test` か `browser-test`) で確かめた
- [ ] 使っているアダプタの情報を返す `adapter_name()` (wgpu の `AdapterInfo::name`。WebGPU では `GPUAdapterInfo` の `description` か
      `vendor`/`architecture` から wgpu が作る文字列。WebGL2 では `WEBGL_debug_renderer_info` が使えればその名前、無ければ空) を足した。
      空のときの扱い (画面に何を出すか) は呼ぶ側に任せ、doc コメントに書いた
- [ ] `@ugoe/player` の `SurfaceWasm`・`WebPuppet` の型と、`PuppetSurface`/`Player` に電力の希望を渡す口 (作るときと作り直すときに同じ値を使う) を足した。
      既定 (`"default"`) では今と同じ振る舞い
- [ ] docs/portability.md (か ugoe-wasm の README) の WebPuppet の作り方の説明に、`powerPreference` は希望で、ブラウザと OS で効き方が違うこと
      (T0076 のメモの表の要約と出典) を書いた
- [ ] `just wasm-check`・`just crate ugoe-wasm` と `just ci` が通る

### 確かめていないこと (実機)

- 2 つの GPU を持つ実機 (Windows のノート PC・Intel の Mac) で、希望を変えると使う GPU が変わるか (Windows の WebView2 では変わらない見込み)

## 参考

- crates/ugoe-wasm/src/web.rs (`create`・`create_with`・`backend`)、crates/ugoe-wasm/src/web/lost_context.rs
- packages/player/src/surface.ts・wasm.ts、bindings/web/src/runtime.ts (`<ugoe-avatar>` は今は触らない。要るなら別のタスク)
- WebGPU の `powerPreference`: https://gpuweb.github.io/gpuweb/#dom-gpurequestadapteroptions-powerpreference
- WebGL の `powerPreference` と、2 回目の `getContext` が属性を無視すること: https://registry.khronos.org/webgl/specs/latest/1.0/#5.2
- T0076 のメモ (調べた結果)

## メモ

- T0076 で起票した (利用者の要望 2026-10-08「GPU の優先度」の 1. 使う GPU を選ぶ)。T0860 (スタジオの GPU の設定) がこれを使う。
- 引数を足すのは ugoe-wasm の公開 API の追加 (省ける引数) で、既存の呼び出しは変わらない。`@ugoe/runtime` (bindings/web) の
  `createPuppet` の options に足すかは、要望があれば別のタスクにする (起票の候補)。
- 依存の追加・unsafe・OS の設定の書き換えは要らないので、判断は要らない。
