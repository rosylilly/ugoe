---
id: T0475
title: "wasm: WebPuppet の作成が WebGL のコンテキストを取れないときに glow の中で panic する"
status: done
milestone: M9
crates: [ugoe-wasm, ugoe-render, packages/player, bindings/web, docs]
depends_on: []
assignee: claude-worker-T0475
---

# T0475: wasm: WebPuppet の作成が WebGL のコンテキストを取れないときに glow の中で panic する

## 目的

T0938 の browser-test (`just player-browser`) を流している最中に、ugoe-wasm が glow の中で panic した
(`glow-0.17.0/src/web_sys.rs:285` の `Option::unwrap`)。ブラウザの WebGL のコンテキストの数の上限 (16) に当たっていた最中で、
コンテキストを作れなかったことが原因と見込まれる (T0938 の候補。確かめていない)。
`WebPuppet.create` は、作れないときに panic (wasm の trap で以後の呼び出しが壊れる) ではなく、コードのある例外で返すべき。

## 受け入れ条件

- [x] 再現の手順をメモに書いた (コンテキストの上限を下げた Chrome、作成と破棄の繰り返しなど)。panic の起きる経路 (wgpu → glow の呼び出し) を特定した
- [x] `WebPuppet.create` (と作り直しの経路) が、コンテキストを作れないときに panic せず、`name`・`code` のある例外 (T0434・T0465 の形に揃える) を返す。上流の glow の不具合なら、呼ぶ前に確かめて避けるか、上流へ報告する案をメモに
- [x] `@ugoe/player` の PuppetSurface と `<ugoe-avatar>` が、その例外を作り直しの失敗として扱う (T0938 の回数に数える)。browser-test にシナリオがある
- [x] `just wasm-check`・`just player-browser`・`just conformance`・`just ci` が通る

## 参考

- T0938 のメモ (起票の候補)、packages/player/browser-test/run.mjs (`--max-active-webgl-contexts`)
- crates/ugoe-wasm/README.md、docs/ui.md#webpuppet-の寿命

## メモ

- オーケストレーターが直接起票した (T0938 の候補。バグなので候補の一覧を待たずに起票)。
- 再現 (2026-10-05):
  - panic の経路: `WebPuppet.create` → `wgpu::Instance::create_surface` (wgpu-hal 30 の `gles/web.rs` の `create_surface_from_canvas` が
    `canvas.getContext("webgl2", {antialias: false})` を受け取る。`null` は `InstanceError` で返すが、**失ったコンテキストは確かめない**) →
    `request_adapter` → wgpu-hal の `enumerate_adapters` → `glow::Context::from_webgl2_context` → glow の `build_extensions!` の
    `context.get_supported_extensions().unwrap()` (`glow-0.17.0/src/web_sys.rs:285:50`)。失ったコンテキストの `getSupportedExtensions()` は `null` を返す。
    panic の後、作成の Promise は決着しない (待ち続ける)。
  - 確実な再現: canvas の `getContext("webgl2", {antialias: false})` を先に作って `WEBGL_lose_context` の `loseContext()` で失わせ、その canvas で
    `WebPuppet.create(canvas, "webgl2" | "auto")` を呼ぶ (browser-test の `create-on-lost-context`。修正の前は panic して時間切れで落ちた。
    コミット `test(player): reproduce the wasm create panic ...` が失敗するテスト)。同じ状態で `PuppetSurface.attach`・作り直しも試す (`attach-on-lost-context`・`recreate-on-lost-context`)。
  - コンテキストの上限: run.mjs に `--max-webgl-contexts N` を足し、Chrome の既定の 16 で全シナリオを流したが、今のシナリオでは panic は
    起きなかった (修正の前の wasm で。新しいシナリオの 3 つだけが上の理由で落ちた)。別に、上限 4 の Chrome で 8 つのコンテキストを続けて作ると、
    失われるのは古いものから (同期的に `isContextLost()` が真) で、新しく作ったものが失われた状態で返ることは無かった。T0938 で起きたときに
    どのコンテキストが失われた状態で返ったか (前に失った canvas の取り直し・描画バッファを作れないときなど) は確かめていない。
    どの理由でも「`getContext` が失ったコンテキストを返す」ことで同じ経路を通るので、そこで確かめて避けた。
- やったこと:
  - ugoe-wasm: `src/create.rs` に `CreateError` (`name` が `"CreateError"`。`backend_unavailable`・`context_unavailable`・`adapter_unavailable`・
    `device_unavailable`。補足の値なし) を置き、`WebPuppet.create` の GPU・ブラウザの都合の失敗をこの形で返す (T0434・T0465 と同じ `coded_error`)。
    WebGL2 で作るとき (WebGPU を使わないとき) は、サーフェスを作った直後・アダプタを求める前に、同じ canvas の `getContext("webgl2")` の
    `isContextLost()` を確かめ、失っていれば `context_unavailable` を投げる (glow へ渡さない)。知らない `backend` などは今までどおり `code` の無い `Error`。
    `codes.json` に `create_error` を足した。web-sys の feature に `WebGl2RenderingContext` を足した。
  - `@ugoe/runtime`: `src/create-errors.ts` (`CREATE_ERROR_CODES`・`UgoeCreateErrorCode`・`UgoeCreateError`・`isCreateError`) とテスト、
    `codes.test.ts` の突き合わせ。`<ugoe-avatar>` は作成の例外をもともと作り直しの失敗として扱っている (待って試し直し、回数に数える) ので
    コードは変えず、テストの偽物が `CreateError` を投げるようにし、最初の作成の `CreateError` が `error`・`ready` に届くテストを足した。
  - `@ugoe/player`: `PuppetSurface` も作成の例外をもともと同じく扱っているのでコードは変えず、browser-test に 3 つのシナリオを足した
    (`create-on-lost-context`・`attach-on-lost-context`・`recreate-on-lost-context`。最後のものは、作れなかった作り直しが回数に数えられ、
    `MAX_RESTARTS` で諦めることまで見る)。
  - 文書: crates/ugoe-wasm/README.md (`作成の失敗 (T0475)`)・bindings/web/README.md・docs/ui.md#webpuppet-の寿命・docs/testing.md・packages/player/README.md。
- 確かめたこと: `just wasm-check`・`just crate ugoe-wasm`・`just player-browser` (13 のシナリオすべて。page.log に panic なし)・`just conformance`
  (wasm-webgpu も ok。WebGPU の経路は変えていない)・`pnpm --filter @ugoe/runtime test`・`just ci` (main に rebase した後に流して通った。ほかは rebase の前)。
- 確かめていないこと: 本物の GPU で、コンテキストの上限・GPU プロセスの不調のときに Chrome が失ったコンテキストを返す場面 (SwiftShader で
  `loseContext` で作った状態だけ)。Firefox・Safari。WebGPU の `adapter_unavailable`・`device_unavailable` を実際に起こすこと (コードを付けただけ)。
- 上流への報告の案 (投稿はしていない):
  - wgpu (wgpu-hal の GLES の web): `create_surface_from_context` か `enumerate_adapters` で `WebGl2RenderingContext::is_context_lost()` を確かめ、
    失っていれば `InstanceError` を返す (または空のアダプタの一覧)。`Adapter::new_external` の doc の「失っていないこと」を、canvas からの経路でも確かめる。
  - glow: `from_webgl2_context`・`from_webgl1_context` の `get_supported_extensions().unwrap()` と `get_parameter(VERSION)` の `as_string().unwrap_or_else(panic)` が、
    失ったコンテキストで panic する。`Result` を返す版を足すか、失ったコンテキストを空の拡張の一覧として扱う。
  - どちらかが直ったら、ugoe-wasm の `webgl2_context_lost` は消せる (`context_unavailable` は wgpu の `create_surface` か `request_adapter` の失敗から付く)。
- 起票の候補:
  - `@ugoe/player` とエディタで `CreateError` を訳す (今は `errors.renderStart` に英語の `message` を埋めて出す。`context_unavailable` なら
    「GPU のコンテキストを作れません。ほかのタブを閉じるか、少し待って作り直してください」のような文)。T0940 (`RenderError` を訳す) と同じ形。
  - 作成の後 (アダプタを求めた後・`configure` の途中など) に WebGL2 のコンテキストを失ったときにも glow・wgpu の中で panic しないか
    (今の確かめは作成の入口だけ。作成は同期的に進むので、JS からの喪失は入らないが、GPU プロセスの喪失の時機は確かめていない)。
