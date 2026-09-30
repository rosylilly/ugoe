---
id: T0404
title: npm パッケージと Web Components
status: done
milestone: M7
crates: [ugoe-wasm, bindings/web, docs, pnpm-workspace.yaml, pnpm-lock.yaml, justfile, biome.json, .gitignore, scripts, .github/workflows, AGENTS.md]
depends_on: [T0402]
assignee: claude-worker-T0404
---

# T0404: npm パッケージと Web Components

## 目的

`@ugoe/runtime` (wasm + TypeScript の型) と `<ugoe-avatar>` Web Component を作る。

## 受け入れ条件

- [x] `npm i @ugoe/runtime` 相当のローカルインストールで、モデルを読み込んで描画できるサンプルがある
- [x] `<ugoe-avatar src="...">` でパラメータを属性や JS から設定できる
- [x] WebGPU と WebGL2 の両方で動く
- [x] ライセンスは MIT (利用者の決定、2026-10-01)。GPL-3.0-only の `packages/` とは分け、`bindings/web` などに置く。ライセンスの境界と置き場所を ADR 0025 に書き、ADR 0018/0019 との関係を示す。`just licenses` が npm の依存も検査するなら、MIT と両立しない依存を入れない
- [x] `just ci` が通る

## 参考

- `docs/adr/0017-single-runtime-distribution.md`
- `docs/portability.md#配布`

## メモ

- **置き場所とライセンス**: `bindings/web/` (`@ugoe/runtime`、MIT、`private: true`)。GPL の `apps/`・`packages/` は import も写しもしない
  (ビューアは読んだだけ)。境界は ADR 0025 (`docs/adr/0025-web-runtime-package.md`)。AGENTS.md のライセンスの行と構成にも `bindings/` を足した。
- **構成**: `src/index.ts` (入口)・`runtime.ts` (`initRuntime` / `createPuppet` / `loadModel` / `fetchModel`)・`avatar.ts`
  (`<ugoe-avatar>` の `UgoeAvatarElement` と `defineUgoeAvatar`)・`element.ts` (登録するだけの入口 `@ugoe/runtime/element`)・
  `attributes.ts` (属性の解釈。Vitest でテスト)。wasm の JS バインディングは `src/wasm/` に生成 (gitignore) し、`dist/wasm/` に写して配る。
  生成物が無い環境の型検査用に、手書きの型 `stub/ugoe_wasm.d.ts` を置いた (`scripts/check.mjs` が無いときだけ使う)。
- **`<ugoe-avatar>`**: 属性 `src`・`backend` (`auto` で WebGPU の作成に失敗したら新しい canvas の WebGL2)・`parameters`
  (`"AngleX: 15; EyeLOpen: 0.5"`。パラメータの ID は大文字小文字を区別するので 1 つの属性にまとめた)・`expression`・`motion`
  (+`motion-priority`)・`paused`。JS は `setParameter` / `getParameter` / `parameters` (代入可) / `parameterDefinitions` / `load` /
  `ready` / `puppet` (WebPuppet そのもの) / `requestRender`、イベントは `load` / `error`。ResizeObserver で
  `devicePixelContentBoxSize` に合わせ、画面外と `paused` の間は rAF を止める。GPU を失ったら作り直す (3 回まで)。
  `backend` の変更と作り直しでは、モデルを読み直してパラメータの値を引き継ぐ。
- **ugoe-wasm の修正**: WebGPU の canvas の背景が黒になっていた。wgpu 30 は WebGPU のサーフェスの alpha を `Opaque` だけと報告するため。
  WebGPU では報告によらず `PreMultiplied` にした (`src/web.rs`)。エディタ・ビューアの WebGPU の表示も透明な背景になる。
  ビューア (`vite build` したもの) を WebGPU (SwiftShader の Vulkan) と WebGL2 で開き、見本「アバター」を背景色 #3366aa の不透明と
  「透明」(市松模様) の両方で撮った。どちらの API でも背景とモデルの縁 (髪の先の半透明) が同じに見え、黒い縁や暗いにじみは無い
  (scratchpad の `T0404/shots/viewer-{webgpu,webgl2}-{opaque,transparent}{,-edge}.png`)。エディタの WebGPU 表示は確かめていない。
  `just test-gpu` はネイティブの描画なので、この変更 (wasm の canvas の設定だけ) では実行していない。
- **検証したこと**: `pnpm --filter @ugoe/runtime check` (生成物あり・スタブの両方)・`test`・`build` (`just web-runtime`)、
  `pnpm pack --out ugoe-runtime.tgz` → `examples/basic` に `npm install` (npm i 相当) → `serve.mjs` で配り、ヘッドレス Chrome
  (Playwright、SwiftShader) で:
  - WebGL2 (`?backend=webgl2`)・WebGPU (`--enable-unsafe-webgpu --use-vulkan=swiftshader` で `?backend=webgpu`)・`auto`
    (WebGPU なしの Chrome では WebGL2、WebGPU ありでは WebGPU) のそれぞれで、`<ugoe-avatar>` と `createPuppet` の両方が描画した。
  - 属性の `parameters` の変更と、JS の `setParameter` / `parameters =` で値が変わり絵が変わる。未知の ID は `false`。
  - `backend` の切り替え (webgl2 → webgpu) で作り直し、パラメータの値を引き継ぐ。未知の `expression` で `error`、取得できない `src` で `error`。
    `src` の変更・`load(bytes)`・外して戻す (作り直して読み直す) も確かめた。
  - Vite 8 でバンドルしたアプリ (tarball を入れたもの) でも、wasm が資産として出力され、描画できた。
  - スクリーンショット: scratchpad の `T0404/shots/` (`webgl2-page.png`・`webgpu-page.png`・`*-js-params.png`・`vite.png` など)。
- main (T0651・T0518・T0714 の後) に rebase した後で、`just wasm`・ビューア用の `just wasm`・`just ci`・`just wasm-check` を通した。
- **確かめていないこと**: 本物の GPU の WebGPU (SwiftShader の Vulkan だけ)、Firefox・Safari、`auto` で WebGPU の作成に失敗して
  WebGL2 に切り替える経路 (失敗を起こせなかった)、`motion` 属性 (見本のバンドルにモーションが無い。`start_motion` を呼ぶだけ)、
  GPU を失ったときの作り直し。npm への公開はしていない。
- **範囲外の変更 (team-lead の承認済み。crates: に足した)**:
  - `pnpm-workspace.yaml` に `bindings/*`、`pnpm-lock.yaml`: パッケージの devDependencies (typescript・vitest) を入れ、
    `pnpm --filter @ugoe/runtime` で動かすため。`pnpm install --offline` で済んだ (新しく取りに行くパッケージは無い。ほかのパッケージと同じ版)。
  - `justfile`: `web-check` に `@ugoe/runtime` の check/test を足し (`just ci` で検査)、パッケージを作る `web-runtime` レシピを足した。
  - `biome.json`: `bindings/**` を検査の対象にし、生成物の `bindings/web/src/wasm` を外した。
  - `scripts/check-licenses.mjs`: bindings/web の `license` が MIT で、`dependencies` が寛容なライセンスで使えることを検査する
    (今は npm を見ていなかったため)。クレートの検査はそのまま (末尾に足しただけ)。GPL にしたときと未知の依存を足したときに失敗することを確かめた。
  - `.github/workflows/ci.yml`: web ジョブで `@ugoe/runtime` の test と、wasm の生成後の build/check (本物の型で検査)。
  - `AGENTS.md`: ライセンスの行とリポジトリ構成に `bindings/` (MIT) を足した。
  - `.gitignore` はルートを変えず、`bindings/web/.gitignore` (生成物・tarball・見本の package-lock.json) で足りた。
- **後続の候補**: npm への公開とリリースの自動化 (版をクレートと揃える)、wasm の縮小 (約 8MB・gzip 約 2MB。`opt-level = "z"`・wasm-opt)、
  `<ugoe-avatar>` のフェイストラッキング・視線の追従 (MIT で書き直す必要がある。`packages/face-tracking` は GPL)、適合テスト (T0408) への組み込み、
  バンドルにモーションを含む見本での `motion` 属性の確認。
