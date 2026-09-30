# @ugoe/runtime

ugoe のランタイムを Web で使うための npm パッケージです。中身は次の 3 つです。

- ugoe-wasm (`crates/ugoe-wasm`) の wasm と JS バインディング (`WebPuppet`) と、その TypeScript の型
- 初期化・作成・読み込みの薄い補助 (`initRuntime`・`createPuppet`・`loadModel`・`fetchModel`)
- Web Component の `<ugoe-avatar>`

WebGPU が使えれば WebGPU、使えなければ WebGL2 で描きます。ライセンスは **MIT** です
([ADR 0025](../../docs/adr/0025-web-runtime-package.md)。GPL-3.0-only の `packages/` とは別の場所に置いています)。

> まだ npm には公開していません (`private: true`)。下の「ローカルで使う」の手順で tarball を作って入れます。
> 公開の手順と自動化は「[リリース](#リリース)」にあります。

## `<ugoe-avatar>`

```html
<script type="module">
  import "@ugoe/runtime/element"; // <ugoe-avatar> を登録する
</script>

<ugoe-avatar src="avatar.ugoe" parameters="AngleX: 15; EyeLOpen: 0.5" style="width: 400px; height: 400px"></ugoe-avatar>
```

| 属性 | 内容 |
| --- | --- |
| `src` | モデルの URL。`.ugoe` バンドル (先頭が `UGOE`) か、モデルの JSON |
| `backend` | `auto` (既定) / `webgpu` / `webgl2`。`auto` で WebGPU の作成に失敗したら、新しい canvas の WebGL2 で作り直す。変えると作り直す (モデルは読み直し、パラメータの値は引き継ぐ) |
| `parameters` | `"ID: 値; ID: 値"` (区切りは `;` か改行、`:` の代わりに `=` も可)。読み込みのたびと、属性を変えたときに設定する。属性から消した ID は元に戻さない |
| `expression` | バンドルの表情の名前。消すと表情を止める |
| `motion` | バンドルのモーションの名前。消すとモーションを止める。再生中のモーションより優先度が低くて始められなければ、始められるまで毎フレーム試す (待機モーションは、ほかのモーションが終わってから流れる) |
| `motion-priority` | `idle` (既定) / `normal` / `force` |
| `paused` | 時間を止める (物理・モーション・目パチ・呼吸を進めない)。パラメータを変えたときだけ描き直す |

パラメータの ID は大文字と小文字を区別します (HTML の属性名は小文字になるので、パラメータごとの属性にはしていません)。

```js
const avatar = document.querySelector("ugoe-avatar");
avatar.addEventListener("load", (e) => console.log(e.detail.backend, e.detail.warnings));
avatar.addEventListener("error", (e) => console.error(e.detail.error));

avatar.setParameter("AngleX", -20);       // 値域にクランプ。ID が無ければ false
avatar.parameters = { AngleY: 10, BodyAngleX: 5 };
avatar.parameters;                         // { AngleX: -20, AngleY: 10, ... } (今の値)
avatar.parameterDefinitions;               // [{ id, name, min, max, default, ... }]
await avatar.load(bytesOrJsonString);      // src の代わりに直接読み込む。戻り値は警告
const puppet = await avatar.ready;         // WebPuppet そのもの (下)
puppet.set_lighting_json(JSON.stringify({ ambient: [0.5, 0.5, 0.5] }));
avatar.requestRender();                    // puppet を直接操作したら描き直しを頼む
avatar.setTracking({ blendshapes: { jawOpen: 0.4 }, head: { yaw: 10, pitch: 0, roll: 0 } }); // 下の「フェイストラッキング」
avatar.clearTracking();
```

- 大きさは要素の CSS の大きさ (既定 300x300) × `devicePixelRatio` です。背景は透明なので、要素の `background` がそのまま見えます。
- 画面外にあるとき (IntersectionObserver) と `paused` の間は `requestAnimationFrame` を回しません。
- GPU を失ったら (GPU プロセスのクラッシュなど)、新しい canvas で作り直してモデルを読み直します (パラメータの値は引き継ぐ。3 回まで)。
- ページから外すと GPU の資源を捨て、戻すと作り直します。
- 別の名前で登録するなら、`@ugoe/runtime/element` の代わりに `defineUgoeAvatar("my-avatar")` を呼びます。

## フェイストラッキング

このパッケージはカメラもトラッカーも持たず、**入力を渡す口だけ**を持ちます
([ADR 0025](../../docs/adr/0025-web-runtime-package.md#フェイストラッキング-t0410))。トラッカー (MediaPipe Face Landmarker など) は
ホストのアプリが用意し、1 フレームごとの結果を `setTracking` に渡します。ブレンドシェイプからパラメータへの対応付けは
ランタイム (Rust) が行います ([docs/spec/avatar-profile.md](../../docs/spec/avatar-profile.md))。

```js
import { FaceLandmarker, FilesetResolver } from "@mediapipe/tasks-vision"; // ホストが入れる (Apache-2.0)
import { trackingFromFaceLandmarker } from "@ugoe/runtime";

const vision = await FilesetResolver.forVisionTasks("https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision/wasm");
const landmarker = await FaceLandmarker.createFromOptions(vision, {
  baseOptions: { modelAssetPath: "face_landmarker.task" },
  runningMode: "VIDEO",
  outputFaceBlendshapes: true,
  outputFacialTransformationMatrixes: true,
});
function track() {
  const result = landmarker.detectForVideo(video, performance.now());
  const input = trackingFromFaceLandmarker(result); // 顔が無ければ null
  if (input) avatar.setTracking(input);
  requestAnimationFrame(track);
}
```

- `setTracking(input)` の `input` は `{ blendshapes: { 名前: 0〜1 }, head: { yaw, pitch, roll }, body? }` (度)。
  名前は ARKit のブレンドシェイプの名前 (MediaPipe のカテゴリ名と同じ)、左右と回転の符号はトラッキングされた人自身から見たもの
  ([向きと左右の約束](../../docs/spec/avatar-profile.md#向きと左右の約束))。ほかのトラッカーでも、この形にすれば渡せます。
- 入力は次のフレームから、モーションの後・表情と目パチの前に書き込まれます (`paused` の間も反映)。トラッキングの間は
  モデルの目パチを止め、モデルを読み直しても最後の入力を渡し直します。`clearTracking()` でやめると、トラッキングが書き込んでいた
  パラメータを既定値 (`parameters` 属性にあればその値) に戻し、目パチを戻します。
- `trackingFromFaceLandmarker(result, { mirrored: true })`: トラッカーに**左右反転した映像**を入れたときに使います
  (左右の名前を入れ替え、ヨーとロールの符号を反転する)。表示だけを反転しているなら不要です。変換は `mirrorTracking` と
  `headRotationFromMatrix` (顔の変換行列 → 頭の回転) としても使えます。
- 平滑化 (揺れを抑える)・キャリブレーション (利用者の中立の顔を差し引く)・顔を見失ったときの扱いはホストが行います。
- 見本 (`examples/basic`) の「tracking (合成の入力)」は、時間の関数で作った入力を毎フレーム渡します (カメラを使わない)。

## `WebPuppet` を直接使う

```js
import { createPuppet, fetchModel } from "@ugoe/runtime";

const canvas = document.querySelector("canvas");
canvas.width = canvas.clientWidth * devicePixelRatio;
canvas.height = canvas.clientHeight * devicePixelRatio;
const puppet = await createPuppet(canvas, { backend: "auto" }); // wasm の初期化もする
const warnings = await fetchModel(puppet, "avatar.ugoe");        // または loadModel(puppet, bytes)
puppet.set_parameter("AngleX", 15);
let last = performance.now();
requestAnimationFrame(function frame(now) {
  puppet.update(Math.min((now - last) / 1000, 0.1));
  last = now;
  puppet.render();
  requestAnimationFrame(frame);
});
```

`WebPuppet` の API (物理・モーション・表情・トラッキング・ライティング・着せ替えなど) は
[crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md) を見てください。

### wasm の場所

`dist/wasm/ugoe_wasm_bg.wasm` を `import.meta.url` からの相対で読みます。Vite などのバンドラはこれを資産として扱います。
別の場所に置くときは、最初に `initRuntime({ wasm: "/assets/ugoe_wasm_bg.wasm" })` を呼びます。
サーバーは `.wasm` を `application/wasm` で返してください。wasm は約 4.5MB (gzip で約 1.3MB、brotli で約 0.96MB) です
([wasm の大きさ](#wasm-の大きさ))。

### WebGPU と WebGL2

`backend: "auto"` (既定) は WebGPU のアダプタを要求し、無ければ同じ canvas で WebGL2 を使います。アダプタはあってもデバイスの
作成などに失敗したときは、`createPuppet` が例外を投げます。1 つの canvas は WebGPU と WebGL2 を併用できないので、
**新しい canvas** と `backend: "webgl2"` で作り直してください (`<ugoe-avatar>` は自分でそうします)。

WebGL2 への切り替えは、見本のページで次のように試せます (WebGPU が使えるブラウザで)。

| URL | 真似る状況 | 期待する結果 |
| --- | --- | --- |
| `?simulate=no-adapter` | WebGPU のアダプタが無い (`navigator.gpu.requestAdapter` が `null`) | `auto` が同じ canvas で WebGL2 を使う |
| `?simulate=device-error` | アダプタはあるがデバイスの作成に失敗する (`GPUAdapter.prototype.requestDevice` が失敗) | `<ugoe-avatar>` は警告 (`falling back to WebGL2`) を出して新しい canvas の WebGL2 で作り直す。`createPuppet` の例も同じく作り直す |
| `?backend=webgl2` | — | 最初から WebGL2 |

自分のページで試すときも、`WebPuppet` を作る前に同じように `navigator.gpu` を書き換えれば同じ経路を通ります。
描画に使っている API は `load` イベントの `detail.backend` か `puppet.backend()` で分かります。

## ローカルで使う (npm i 相当)

リポジトリのルートで:

```sh
cargo install wasm-bindgen-cli --version "$(just wasm-bindgen-version)" --locked  # 初回だけ
just web-runtime                     # wasm と TypeScript をビルドして bindings/web/dist/ に出す
cd bindings/web && pnpm pack --out ugoe-runtime.tgz
```

できた `ugoe-runtime.tgz` を `npm i ./ugoe-runtime.tgz` で入れます。

### 見本 (`examples/basic`)

```sh
cd bindings/web/examples/basic
npm install        # ../../ugoe-runtime.tgz を node_modules/@ugoe/runtime に入れる
npm start          # http://localhost:8080/ (?backend=webgl2 で WebGL2 に固定)
```

パッケージを作り直したら、`rm -rf node_modules package-lock.json && npm install` で入れ直します
(npm は同じ名前の tarball の中身の変化を見ないため)。

`<ugoe-avatar>` (属性・スライダーからの `setParameter`・表情とモーションの切り替え・`backend` の切り替え・合成のトラッキング) と、
`createPuppet` で canvas に直接描く例を並べています。モデルはリポジトリの `assets/samples/` (MIT) を配ります。
URL の `?src=` で `<ugoe-avatar>` のモデルを、`?simulate=` で WebGPU の失敗を ([上](#webgpu-と-webgl2)) 変えられます。

リポジトリの見本のモデルはモーションと表情を持たないので、`motion` / `expression` 属性を試すには、モーション
(`motions/`: 待機の `idle`・うなずく `nod`・首を振る `shake`) と表情 (`expressions/`: `smile`) を足したバンドルを作ります。

```sh
npm run sample:motion   # generated/avatar-motion.ugoe を作る (ugoe-cli を cargo で動かす。リポジトリの中で)
npm start               # http://localhost:8080/?src=generated/avatar-motion.ugoe
```

## リリース

npm への公開は、パッケージを使う人 (このリポジトリを持つ人) が決めて行います。いまは `private: true` なので、
どの手順でも公開はされません。

### 版

- 版の正はルートの `Cargo.toml` の `[workspace.package] version` です。すべての配布物で版を揃えます
  ([docs/portability.md](../../docs/portability.md#配布))。`package.json` の版はそれに合わせます。
- `node scripts/release.mjs sync` で `package.json` の版をクレートの版にし、`node scripts/release.mjs check [v<版>]` で確かめます
  (単体テストも、揃っていなければ失敗します)。
- プレリリース (`0.2.0-alpha.1` など) は npm の dist-tag `next`、それ以外は `latest` で公開します。

### 公開できるようにする (初回だけ)

1. npm に `@ugoe` のスコープ (組織) を用意します。
2. `package.json` から `"private": true` を消してコミットします (消すかどうかは利用者の判断です)。
3. CI から公開するなら、次のどちらかを用意します。
   - npm の **trusted publishing**: npm のパッケージの設定で、GitHub のこのリポジトリとワークフロー
     `web-runtime-release.yml` を信頼する発行元にします (トークンは要りません)。最初の版は手で公開する必要があります。
   - **トークン**: npm の automation トークン (公開の権限) を、リポジトリのシークレット `NPM_TOKEN` に置きます。

### 公開する

- **CI (推奨):** クレートの版を上げたコミットに `v<版>` のタグを付けて push します。`.github/workflows/web-runtime-release.yml` が、
  タグ・クレート・`package.json` の版が揃っているかを確かめ、wasm と TypeScript をビルドし、型検査・テスト・ライセンスの検査の後、
  来歴 (provenance) を付けて `npm publish` します。手で起動したとき (workflow_dispatch) は、既定で `--dry-run` です。
- **手元:** リポジトリのルートで次を実行します。

  ```sh
  just web-runtime                                  # dist/ を作る
  cd bindings/web
  node scripts/release.mjs check v0.1.0             # 版を確かめる
  node scripts/release.mjs publish --dry-run        # 入る内容を確かめる
  npm login && node scripts/release.mjs publish     # 公開する
  ```

`private: true` の間の `publish` は、公開せずに入る内容 (`npm pack --dry-run`) だけを表示して成功します。

## wasm の大きさ

`pnpm --filter @ugoe/runtime build` (`just web-runtime`) は、エディタとビューアの `just wasm` とは別の設定で wasm を作ります
(`scripts/build.mjs`)。ランタイムの機能は切り分けていません (エディタとビューアの wasm と同じ API)。

| | 大きさ | gzip (-9) | brotli |
| --- | --- | --- | --- |
| 前 (`just wasm` と同じ: release、lto = "thin") | 7,920,320 | 1,974,207 | 1,310,795 |
| いま | 4,452,501 | 1,300,532 | 964,480 |

(バイト。T0410 の時点。gzip と brotli は Node の zlib で測った)

- cargo のプロファイル `web-dist` を `--config` で定義して使います: `opt-level = "z"`・`lto = "fat"`・`codegen-units = 1`。
  ただし毎フレーム動くクレート (`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe-render`・`ugoe`・`ugoe-core`・`glam`) は
  `opt-level = 3` に戻します。すべてを `"z"` にすると 4,354,077 バイト (gzip 1,250,656) とさらに小さいものの、
  CPU の評価 (`update` と変形) が約 2.5 倍遅くなったためです (見本のアバターで 1 フレーム約 19µs → 48µs。ヘッドレスの Chrome)。
- wasm-bindgen の `--remove-name-section` で関数名の節 (約 1.8MB。パニックのスタックトレースに名前が出るだけ) を外します。
- **wasm-opt (binaryen) は使いません。** `-Oz` で生の大きさは約 1 割減りますが、gzip した大きさはかえって約 1 割増えました。
- 残りの大部分は MessagePack の読み込み (`rmp_serde`、約 0.8MB)・naga (シェーダの変換、約 0.55MB)・wgpu です。
  機能の切り分けで小さくするなら、まず `rmp_serde` の読み込みのコード (モデルの型ごとの展開) を見直すのがよさそうです (未着手)。

## 開発

| コマンド | 内容 |
| --- | --- |
| `pnpm --filter @ugoe/runtime build` | = `just web-runtime`。wasm のビルド (`src/wasm/` に生成、gitignore 済み)・`tsc`・`dist/` への出力 |
| `pnpm --filter @ugoe/runtime check` | 型検査。`src/wasm/` が無ければ手書きの型 `stub/ugoe_wasm.d.ts` で検査する |
| `pnpm --filter @ugoe/runtime test` | 属性の解釈などの単体テスト (Vitest) |

- `crates/ugoe-wasm` の公開 API を変えたら、`stub/ugoe_wasm.d.ts` を生成物 (`src/wasm/ugoe_wasm.d.ts`) から作り直します。
- このパッケージは MIT です。`apps/`・`packages/` (GPL-3.0-only) のコードを import したり写したりしないでください ([ADR 0025](../../docs/adr/0025-web-runtime-package.md))。
- 構成: `src/index.ts` (入口)・`src/runtime.ts` (初期化・作成・読み込み)・`src/avatar.ts` (`<ugoe-avatar>`)・
  `src/element.ts` (登録するだけの入口)・`src/attributes.ts` (属性の解釈。DOM と wasm に触れない)。
