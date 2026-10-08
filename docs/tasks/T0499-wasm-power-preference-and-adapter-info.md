---
id: T0499
title: "ugoe-wasm: WebPuppet.create で GPU の電力の希望 (powerPreference) を渡せるようにし、使っているアダプタの名前を返す"
status: done
milestone: M8
crates: [ugoe-wasm, packages/player, docs]
depends_on: []
assignee: claude-worker-T0499
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

- [x] `WebPuppet.create(canvas, backend, powerPreference?)` を足した。`powerPreference` は `"default"`・`"low-power"`・`"high-performance"`
      (省くと `"default"`。ほかの文字列は例外)。WebGPU では wgpu の `PowerPreference` に、WebGL2 では先に作るコンテキストの属性に渡す。
      `antialias: false` は今と同じ (wgpu-hal と同じ属性で作る)
- [x] WebGL2 で先にコンテキストを作っても、作成の途中の喪失の扱い (T0944 の `LostContextGuard`)・`webglcontextlost` の印・作れなかったときの
      `CreateError` のコードが今と同じであることを、既存のブラウザのテスト (`wasm-pack test` か `browser-test`) で確かめた
- [x] 使っているアダプタの情報を返す `adapter_name()` (wgpu の `AdapterInfo::name`。WebGPU では `GPUAdapterInfo` の `description` か
      `vendor`/`architecture` から wgpu が作る文字列。WebGL2 では `WEBGL_debug_renderer_info` が使えればその名前、無ければ空) を足した。
      空のときの扱い (画面に何を出すか) は呼ぶ側に任せ、doc コメントに書いた
- [x] `@ugoe/player` の `SurfaceWasm`・`WebPuppet` の型と、`PuppetSurface`/`Player` に電力の希望を渡す口 (作るときと作り直すときに同じ値を使う) を足した。
      既定 (`"default"`) では今と同じ振る舞い
- [x] docs/portability.md (か ugoe-wasm の README) の WebPuppet の作り方の説明に、`powerPreference` は希望で、ブラウザと OS で効き方が違うこと
      (T0076 のメモの表の要約と出典) を書いた
- [x] `just wasm-check`・`just crate ugoe-wasm` と `just ci` が通る

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

### 作業の記録 (claude-worker-T0499)

- **ugoe-wasm:** `WebPuppet.create(canvas, backend?, power_preference?)` (`"default"` (省いたとき)・`"low-power"`・`"high-performance"`。
  ほかはコードを持たない `Error`。読み取りは `src/create.rs` の `power_preference` でネイティブのテストあり)。WebGPU は
  `RequestAdapterOptions::power_preference` に、WebGL2 はサーフェスを作る前に `getContext("webgl2", { antialias: false, powerPreference })`
  で先に作る (`"default"` でも同じ属性で作る)。`adapter_name()` を足した (WebGPU は `AdapterInfo::name`、WebGL2 は
  `WEBGL_debug_renderer_info` があれば `AdapterInfo::name`、無ければ空。wgpu は無いとき `RENDERER` の一般の名前を入れるので返さない)。
- **公開の API として扱った (`@internal` を付けない):** エディタに限らず、スタジオ (T0860)・ビューア・npm の利用者が GPU を選ぶ・
  使っている GPU を見せるのに使う口なので、T0492 の「エディタ向けの問い合わせ」に当たらない。型の写し `bindings/web/stub/ugoe_wasm.d.ts`
  も直し、`just wasm-types` が生成物と一致することを確かめた。
- **見つけた問題と直したこと:** 先に作る `getContext` の結果を見ずに wgpu に続けて `getContext` させると、GPU プロセスが落ちている間
  (最初の `getContext` が `null`) にすぐ続けて呼んだ `getContext` が、失っていないのに描けないコンテキストを返し、
  `just conformance --only avatar` の `gpu-process-crash-during-create` (delay 0) が「canvas が空」で 4 回続けて落ちた (main では通る。
  前の版の worktree で比べた。属性を付けずに先に作るだけでも落ちたので、属性ではなく 2 回呼ぶことが原因)。先に作る `getContext` が
  `null`・例外なら wgpu に呼ばせずに `context_unavailable` (wgpu-hal と同じ文) にして、今までと同じく最初の `getContext` で決めるようにした。
- **@ugoe/player:** `PowerPreference` 型、`SurfaceWasm.create` の 3 つ目の引数、`PuppetSurface.powerPreference` (作るたび
  (`attach`・GPU を失ったときの作り直し・`restart`) に渡す。変えても今の WebPuppet はそのままで、`restart` から効く)、
  `Player.powerPreference` (同じもの) と `Player.adapterName`。`wasm.ts` の `WebPuppet` に `adapter_name` を足した。
  既定は `"default"` で、WebGL2 では今までと同じ属性 (`antialias: false`、`powerPreference` の既定は `"default"`) のコンテキスト。
- **テスト:** create.rs の名前の読み取り、surface.test.ts と player.test.ts (作成・作り直し・`restart` で同じ値が渡る、`adapterName`)、
  `just player-browser` に `power-preference` を足した (コンテキストの属性が希望どおりで `antialias: false`・描ける・`adapter_name`・
  知らない希望はコードの無い `Error`・失ったコンテキストでは希望を渡しても `context_unavailable`・PuppetSurface の作成と作り直しで渡る)。
  先に作るのをやめると、このシナリオが `low-power` の属性で落ちることを確かめた。SwiftShader の `adapter_name` は
  `"ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)"`。
- **文書:** crates/ugoe-wasm/README.md に「GPU の電力の希望 (T0499)」(T0076 の表の要約と出典)、docs/ui.md の「WebPuppet の寿命」、
  docs/testing.md のシナリオの一覧。
- **`crates:` の外の小さな変更:** `bindings/web/stub/ugoe_wasm.d.ts` (型の写し。`just wasm-types` が要る) と bindings/web/README.md の
  1 文 (`adapter_name()` の案内)。`<ugoe-avatar>`・`createPuppet` には電力の希望を足していない。
- **検査:** `just wasm-check`・`just crate ugoe-wasm`・`just wasm`・`just wasm-types`・`just player-browser` (22 of 22)・
  `just conformance` (すべて ok)・`just ci`。
- **確かめていないこと:** 2 つの GPU を持つ実機 (Windows のノート PC・Intel の Mac) で希望を変えると使う GPU が変わるか。WebGPU で希望を渡す経路
  (ブラウザのテストは WebGL2 で、`requestAdapter` に希望が渡ることは wgpu 30 の backend/webgpu.rs を読んで確かめただけ)。
  エディタのビューポート (`ViewportController`) には希望を渡す口を足していない (既定のまま)。
- **起票の候補:** `@ugoe/runtime` の `createPuppet` の options と `<ugoe-avatar>` の属性に `powerPreference` を足す (T0076 の候補と同じ)。
  エディタのビューポートで GPU を選ぶ・名前を見せる (要望があれば)。

