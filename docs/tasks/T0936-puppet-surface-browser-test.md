---
id: T0936
title: PuppetSurface を本物の wasm とブラウザで確かめる自動テスト (判断が要る: 置き場所)
status: done
milestone: M9
crates: [packages, tests, docs]
depends_on: [T0935]
assignee: claude-worker-T0936
---

# T0936: PuppetSurface を本物の wasm とブラウザで確かめる自動テスト (判断が要る: 置き場所)

## 目的

T0932 の `PuppetSurface` は、本物の wasm とブラウザでは手で書いたページで確かめただけ (T0932 の候補)。自動のテストにする。

**判断が要る:** 置き場所。packages/ は GPL-3.0-only で、`tests/conformance` は MIT。MIT の場所に GPL のコードを入れない (AGENTS.md) ので、
packages/ の中にブラウザのテストを置くか、別の GPL のテストの場所を作るか。

## 受け入れ条件

- [x] 置き場所を決めた (ライセンスの理由を書いた。利用者の判断を待つなら blocked にしてメモに案を書く)
- [x] 本物の wasm とヘッドレスのブラウザで、`PuppetSurface` の作成・描画・作り直し (T0935 の方針) を確かめるテストがあり、`just` のレシピから流せる
- [x] CI で流すか (どのジョブか) を決め、docs/testing.md に書いた
- [x] `just ci` が通る

## 参考

- T0932 のメモ (起票の候補)、T0440 (`<ugoe-avatar>` のブラウザのテスト)

## メモ

- T0026 で起票した (元の候補: T0932 の自動テスト)。作り直しの方針を決める T0935 の後に着手する。

### 決めたこと (置き場所。利用者の判断を待たずに決めた根拠)

- **`packages/player/browser-test/` に置いた (GPL-3.0-only)。** テストは GPL の `src/surface.ts` をそのまま (JS にして) 読み込むので、
  AGENTS.md の「外部のコードとライセンス」から GPL の側に置くことが決まる。`tests/conformance` は MIT。T0915 の `tests/ui-audit` は
  LICENSE を持たず、リポジトリの既定 (ルートの LICENSE。MIT) になるので、そこにも置かない。新しい GPL のテストの場所を作るより、
  対象のパッケージの中に置くほうが小さい (パッケージの LICENSE がそのまま掛かる)。案を並べて選ぶ余地が無かったので blocked にはしていない。
- **Chrome の起動は `just ui-audit` (T0915) と同じ仕組み:** ルートの devDependencies の playwright-core で、システムの Chrome
  (`CHROME` で指定可) を `--use-angle=swiftshader --enable-unsafe-swiftshader` で開く。新しい npm の依存は足していない
  (surface.ts は何も import しないので、`@ugoe/player` の devDependencies の typescript の `transpileModule` で JS にする)。
- **CI:** `conformance` ジョブ (Chrome・wasm-bindgen-cli・pnpm・Rust が既にある) で、適合テストの後に `just player-browser` を流す
  (適合テストが落ちても流す)。`just ci` には入れない (wasm のビルドと Chrome が要る)。

### やったこと

- `packages/player/browser-test/` (`run.mjs`・`index.html`・`scenarios.js`) と `just player-browser [--no-build] [--only …] [--out …]`。
  wasm を `scripts/wasm.mjs build` で `target/player-browser/wasm` に作り、小さなサーバで配る。ページのコンソールは
  `target/player-browser/page.log` に書く (wgpu の警告が多いので画面には出さない)。
- シナリオ (本物の wasm・WebGL2 (SwiftShader)。`WebPuppet.create`・`free` を包んで生きている WebPuppet を数える):
  - `create-draw-detach`: `attach` が `ready`・`created(recovered=false)` 1 回・backend が webgl2・canvas 1 つと `canvasClass`・
    透明でない画素がある・`resize` で描画バッファが CSS の大きさ × 倍率・`detach` で canvas と WebPuppet が残らない。
  - `context-loss-restore`: `loseContext` で `awaitingRestore`・`is_lost()`・`checkLost()` が真、復元までは作り直さない、
    `restoreContext` で新しい canvas に作り直して `created(recovered=true)`・`lost` が呼ばれ、描ける。2 回続けて。
  - `repeated-loss-gives-up`: `MAX_RESTARTS` 回まで作り直し、次で `failed(GpuLostRepeatedlyError)`・canvas と WebPuppet が残らない・
    次の `attach` で作れる。
  - `detach-while-creating`: `WebPuppet.create` を待っている間の `detach` と次の `attach` で `superseded`・遅れてできたものが残らない。
- テストが壊れを見つけることを確かめた: surface.ts の `preventDefault` を消すと喪失の 2 つが落ち、遅れてできた WebPuppet を捨てる
  分岐を消すと `detach-while-creating` が、回数の上限を消すと `repeated-loss-gives-up` が落ちた (戻して 4 つとも通る)。
- 文書: docs/testing.md (種類の表・`just ci` に入れない理由・節「PuppetSurface のブラウザのテスト」)、packages/player/README.md。
- 範囲の外の小さな変更: `justfile` (レシピ 1 つ)、`.github/workflows/ci.yml` (`conformance` ジョブに 1 段と、落ちたときに残す
  `page.log`)。受け入れ条件 (just のレシピ・CI のジョブ) のために要るので行った。

### 確かめていないこと

- CI の上での実行 (push していない)。手元 (WSL2・Google Chrome・SwiftShader) で `just player-browser` が 4 つとも通った。
- WebGPU のデバイスの喪失からの作り直し (ヘッドレスの Chrome で起こす手段が無い。偽の wasm の surface.test.ts だけ)。
- devicePixelRatio だけが変わったときの追従 (ヘッドレスで倍率を途中で変えるには CDP の `Emulation.setDeviceMetricsOverride` が要る)。

### 検査

- `git rebase main` の後に `just ci` (前面、終了コード 0)。ランタイム層は触っていない。

### 起票の候補

- devicePixelRatio の変化を CDP で起こすシナリオを足す (`matchMedia` の `change` が本物のブラウザで届くか)。
- エディタの `ViewportController` と Player を、同じ仕組み (本物の wasm とヘッドレスの Chrome) で確かめるか (今は偽の wasm の Vitest だけ)。
