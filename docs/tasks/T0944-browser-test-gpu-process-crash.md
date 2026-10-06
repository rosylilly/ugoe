---
id: T0944
title: "PuppetSurface のブラウザのテスト: 作成の後に GPU プロセスが落ちても wasm が panic しないことを確かめる"
status: done
milestone: M9
crates: [packages/player, ugoe-wasm, docs]
depends_on: []
assignee: claude-worker-T0944
---

# T0944: PuppetSurface のブラウザのテスト: 作成の後に GPU プロセスが落ちても wasm が panic しないことを確かめる

## 目的

T0475 で、失ったコンテキストを `getContext` が返したときに `WebPuppet.create` が glow の中で panic する不具合を直した。確かめたのは作成の入口だけで、
作成の後 (アダプタを求めた後・`configure` の途中・描画中) に GPU プロセスごとコンテキストを失ったときに glow・wgpu の中で panic しないかは
確かめていない (作成は同期的に進むので JS の `loseContext` は割り込めない。T0475 の候補)。

## 受け入れ条件

- [x] `packages/player/browser-test` に、作成の後に GPU プロセスを落とす (CDP の `Browser.crashGpuProcess` など。ヘッドレスの Chrome と SwiftShader で
      落とせるかを確かめる) シナリオを足し、`PuppetSurface` と `Player` が作り直すか諦めて、page.log に panic が無いことを確かめる。落とせないなら理由と代わりの確かめ方をメモに書いた
- [x] panic が見つかったら ugoe-wasm で失敗として返すように直し、先に失敗するシナリオを入れた (見つからなければメモに書く)
- [x] docs/testing.md#puppetsurface-のブラウザのテスト を直した
- [x] `just player-browser`・`just ci` が通る (wasm を直したら `just wasm-check` も)

## 参考

- T0475 のメモ (確かめていないこと・起票の候補・上流への報告の案)
- packages/player/browser-test/run.mjs、crates/ugoe-wasm/src/create.rs

## メモ

- T0038 で起票した (元の候補: T0475 (作成の後の喪失))。本物の GPU での確かめは docs/manual-check.md の D09。
- GPU プロセスの落とし方 (2026-10-06、Chrome 154.0.8037.57・SwiftShader・ヘッドレス):
  - CDP の `Browser.crashGpuProcess` (Playwright の `browser.newBrowserCDPSession()`) で落とせる。落ちた canvas には `webglcontextlost` が
    届き、約 1 秒後に `webglcontextrestored` が届く。`chrome://gpucrash` は試していない (CDP で足りたため)。
  - Chrome は続けて落ちると GPU を諦め、SwiftShader の後に代わりが無いので 6 回ほどで `GPU process isn't usable. Goodbye.` でブラウザごと
    終わる。また、落ちたときに WebGL を使っていたページの WebGL を止める (以後 `getContext` が `null` → `CreateError` の `context_unavailable`)。
    run.mjs は `--disable-gpu-process-crash-limit`・`--disable-domain-blocking-for-3d-apis` で起動する。
  - run.mjs の `crashGpuProcess(delay)` は、待たずに戻って Node の側から落とすので、wasm の同期的な作成の途中にも落とせる (時機は決まらない)。
- 見つけた panic (どれも作成の途中。panic の後は作成の Promise が決着しない):
  1. glow 0.17 `from_webgl2_context` の `getSupportedExtensions().unwrap()` (`web_sys.rs:285`) と `getParameter(VERSION)` の文字列の `unwrap`
     (`web_sys.rs:293`)。T0475 の確かめ (アダプタを求める前の `isContextLost()`) の後、glow の初期化の途中で失うと起きる。WebGL2 の呼び出しの
     2〜38 回目の前で `loseContext` すると必ず起きた。
  2. wgpu-hal 30 `create_render_pipeline` の `get_uniform_block_index(..).unwrap()` (`gles/device.rs:552`)。GPU プロセスを落としたときだけ起きる:
     リンクできたことを読んだ後に `INVALID_INDEX` が返り、そのとき `isContextLost()` は偽・`getError()` は 0 (JS からは失ったと分からない)。
     作成だけを落としながら 40 回繰り返すと 1 回ほど、120 回で 5 回起きた。`loseContext` では起きない (失ったコンテキストは 0 を返す)。
  - 作成の後 (読み込み・描画・大きさの変更) では、`loseContext` の掃引 (全 1777 回の呼び出しのそれぞれの前で失う) でも、GPU プロセスを
    落としながらの繰り返しでも panic は起きなかった。
- やったこと:
  - ugoe-wasm: `src/web/lost_context.rs` の `LostContextGuard`。WebGL2 で作るとき、wgpu と同じコンテキストのオブジェクトの上で
    `getSupportedExtensions` (`null` → `[]`)・`getParameter` (`VERSION` が文字列でない → `"WebGL 2.0"`)・`getUniformBlockIndex`
    (`INVALID_INDEX` → どのプログラムにも無い番号。続く `uniformBlockBinding` は `INVALID_VALUE` で何もしない) を自分のプロパティで包む。
    生きているコンテキストでは返らない (返れば今も panic する) 値だけを置き換え、`isContextLost()` では見分けない (上の 2)。
    `WebPuppet` の最後のフィールドに持ち、捨てるとプロパティを消す。作成の最後にもう一度 `isContextLost()` を確かめ、途中で失っていれば
    (別の段の失敗も含めて) `context_unavailable` で失敗する (失ったまま作れたことにすると `is_lost()` がまだ偽の WebPuppet を渡すため)。
  - browser-test: wasm の panic (`panicked at`) があればどのシナリオも失敗にする (作成を待つ `within` も panic ですぐ失敗する)。
    シナリオ 5 つ: `lose-context-during-create` (作成の WebGL2 の呼び出しのどの前で失っても `CreateError` の `context_unavailable`。決定的)・
    `invalid-block-index-during-create` (`getUniformBlockIndex` が `INVALID_INDEX` を返しても panic しない。上の 2 を決定的に再現)・
    `gpu-process-crash-during-create` (CDP で時機をずらして落としながら作成・読み込み・描画を繰り返す。決まらない)・`gpu-process-crash`
    (`PuppetSurface` が復元を待って作り直して描く。2 回)・`player-gpu-process-crash` (Player がモデル・モジュール・値を戻して描く)。
    先の 2 つは直す前の wasm で落ちることを確かめた (コミット `test(player): reproduce wasm panics ...`)。
  - 文書: docs/testing.md#puppetsurface-のブラウザのテスト・crates/ugoe-wasm/README.md (作成の失敗)・packages/player/README.md。
- 確かめたこと: `just player-browser` (18 のシナリオすべて。page.log に panic なし)・`just conformance` (wasm・wasm-webgpu ほかすべて ok)・
  `just wasm-check`・`just ci` (main に rebase した後)。直した wasm で、作成だけを落としながら 120 回繰り返して panic 0 回 (`INVALID_INDEX` が
  6〜24 回返った)、作成・読み込み・描画で 120 回繰り返して panic 0 回 (計測用の一時的なシナリオ。残していない)。
- 確かめていないこと: 本物の GPU (ドライバのリセット・GPU プロセスの不調) での喪失 (docs/manual-check.md の D09)。Firefox・Safari の WebGL2。
  WebGPU のデバイスの喪失を `Browser.crashGpuProcess` で起こすこと (ヘッドレスの Chrome で WebGPU を有効にしていない)。
  `gpu-process-crash-during-create` は時機が決まらないので、通っても全ての段を通ったとは言えない (決定的な確かめは先の 2 つ)。
  描画中 (`render` の途中) の GPU プロセスの喪失は、繰り返しでは panic が出なかったが、決まった時機では確かめていない (`loseContext` の掃引だけ)。
- 起票の候補:
  - 上流への報告 (T0475 の案に足す): wgpu-hal の `create_render_pipeline` の `get_uniform_block_index(..).unwrap()` (と `get_shader_storage_block_index`)
    が、GPU プロセスの喪失で `INVALID_INDEX` を受けて panic する。`DeviceError::Lost` を返すのがよい。直ったら `lost_context.rs` の該当する包みを消す。
  - `<ugoe-avatar>` (bindings/web) のブラウザのテストにも GPU プロセスの喪失を足す (今は `@ugoe/player` だけ。wasm は同じなので panic は直っている)。
