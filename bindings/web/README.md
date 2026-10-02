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
| `motion-over-tracking` | 付けると、トラッキング中も、再生中のモーションが動かすパラメータではモーションを優先する (下の「[フェイストラッキング](#フェイストラッキング)」。T0722) |
| `tracking-kept` | `motion-over-tracking` でもトラッキングのままにするパラメータの ID (`"EyeLOpen EyeROpen"`。区切りは空白・`,`・`;`) |
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
- そのため、トラッキングが書き込むパラメータ (頭の向き・目・口など) ではモーションの値が置き換わります。`motion-over-tracking` 属性を
  付けると、トラッキングの値をモーションの前に書き込み、再生中のモーションが動かすパラメータではモーションがその上に乗ります
  (うなずきのモーションはトラッキング中もうなずく。モーションが動かさないパラメータはトラッキングのまま)。`tracking-kept` に並べた
  パラメータは、モーションが動かしてもトラッキングのままです (`tracking-kept="EyeLOpen EyeROpen"` で目だけはトラッキング)。
  ループする待機モーションにも効くので、待機を流しながらトラッキングの頭の向きを見せたいときは、待機の間は属性を外します
  (ビューアは待機以外のモーションの再生中だけ付けています)。`WebPuppet` の `set_motion_over_tracking`・`set_tracking_kept` と同じで、
  C ABI にも同じ口があります ([docs/spec/motion.md](../../docs/spec/motion.md#トラッキング中のモーションの優先))。
- `trackingFromFaceLandmarker(result, { mirrored: true })`: トラッカーに**左右反転した映像**を入れたときに使います
  (左右の名前を入れ替え、ヨーとロールの符号を反転する)。表示だけを反転しているなら不要です。変換は `mirrorTracking` と
  `headRotationFromMatrix` (顔の変換行列 → 頭の回転) としても使えます。
- 平滑化 (揺れを抑える)・キャリブレーション (利用者の中立の顔を差し引く)・顔を見失ったときの扱いはホストが決めます。
  そのための補助 (依存なし・使わなくてよい) を下の「平滑化とキャリブレーション」に置きました。
- 見本 (`examples/basic`) の「tracking (合成の入力)」は、時間の関数で作った入力に揺れを足し、`TrackingFilter` で平滑化して
  毎フレーム渡します (カメラを使わない。`?tracking-filter=off` で平滑化しない)。「camera (MediaPipe)」は本物のカメラの入力を渡します
  ([下](#平滑化の係数を実際の顔で調整する))。

### 平滑化とキャリブレーション

```js
import { TrackingFilter, calibrateTracking, captureNeutral, trackingFromFaceLandmarker } from "@ugoe/runtime";

const filter = new TrackingFilter(); // 値ごとの One Euro フィルタ
let neutral = null; // 「正面を向いて力を抜いてください」の間に集めた入力から作る
let calibrating = true;
const frames = [];

function track() {
  const now = performance.now();
  const input = trackingFromFaceLandmarker(landmarker.detectForVideo(video, now));
  if (!input) {
    filter.reset(); // 顔を見失った: フィルタを忘れる (最後の値を保つ。中立に戻すなら avatar.clearTracking())
  } else if (calibrating) {
    frames.push(input);
  } else {
    avatar.setTracking(filter.apply(neutral ? calibrateTracking(input, neutral) : input, now));
  }
  requestAnimationFrame(track);
}
// キャリブレーションを終えるとき (数秒後など)
neutral = captureNeutral(frames); // 値ごとの平均
calibrating = false;
```

- `new TrackingFilter(options?)` / `apply(input, timeMs)` / `reset()`: 値ごとの One Euro フィルタ
  (止まっているときはカットオフを下げて揺れを抑え、速く動くほど上げて遅れを減らす。Casiez ほか, CHI 2012)。
  回転 (度) と 0〜1 の値は単位が違うので、`{ rotation: { minCutoff, beta, derivativeCutoff }, blendshapes: {...} }` を別に決めます
  (既定は回転 `minCutoff: 1, beta: 0.05`、0〜1 の値 `minCutoff: 1, beta: 2`、`derivativeCutoff` はどちらも 1)。
  入力に無い値 (ブレンドシェイプの名前・体の値) のフィルタは忘れ、次に来たときはその値から始めます。
- `captureNeutral(inputs)`: 中立の入力 (1 つか配列。配列なら値ごとに平均) から基準を作ります。
- `calibrateTracking(input, neutral)`: 頭と体の回転は中立の角度を引きます。ブレンドシェイプは中立の値を 0 として
  `(v − n) / (1 − n)` に引き伸ばします (力を抜いても少し開く口などを 0 にする)。体のピッチと肩の上下は、カメラの高さや体格で
  基準が大きく変わるので、中立にその値が無ければ渡しません。
- これらは `packages/face-tracking` (GPL) とは別に、論文の定義から書いたものです (MIT。ADR 0025)。

### 平滑化の係数を実際の顔で調整する

既定の係数は One Euro フィルタの一般的な値から選んだもので、**本物のカメラと顔ではまだ調整していません** (T0413・T0719)。
見本 (`examples/basic`) の「camera (MediaPipe)」で、カメラの映像を MediaPipe Face Landmarker (CDN から読む。Apache-2.0。
パッケージは MediaPipe に依存しません) で調べて `<ugoe-avatar>` に渡し、係数を画面の欄で変えながら確かめられます。

```sh
cd bindings/web/examples/basic && npm install && npm start
# http://localhost:8080/ を開き、「camera (MediaPipe)」に印を付ける (カメラの許可を求められる)
# 係数は欄か URL で: ?rotation-min-cutoff=1&rotation-beta=0.05&blend-min-cutoff=1&blend-beta=2
# ?tracking-filter=off で平滑化しない入力と比べる
```

欄の下に、頭のヨーの入力と平滑化の後の値、揺れの目安 (直近 60 フレームの、フレームごとの 2 階差分の絶対値の平均。
小さいほど揺れが少ない) を出します。次の順で確かめます。

1. **止まった顔の揺れ**: 正面を向いて動かずにいる間、モデルの頭と目・口が細かく震えないか。震えるなら `minCutoff` を下げる
   (回転なら 0.5 など。揺れの目安の「平滑化」の数字が下がる)。
2. **速く動いたときの遅れ**: 首を素早く振る・口を素早く開け閉めしたとき、モデルが目に見えて遅れないか。遅れるなら `beta` を上げる
   (回転なら 0.1 など)。上げすぎると、動いている間の揺れが戻る。
3. **まばたき**: 目を閉じてすぐ開けたとき、モデルの目が閉じきるか (ブレンドシェイプの平滑化が強すぎると、短いまばたきが半分しか閉じない)。
   閉じきらないなら表情の `beta` を上げるか `minCutoff` を上げる。
4. 部屋の明るさ・カメラ (30fps と 60fps)・顔の距離を変えて、1〜3 を繰り返す。暗いと入力の揺れが大きくなる。

よい値が見つかったら、`src/tracking-filter.ts` の `ROTATION_DEFAULTS`・`BLENDSHAPE_DEFAULTS` とこの README の既定の値、
単体テストを合わせて直します。カメラが無ければ「tracking (合成の入力)」でも揺れの目安を見られます
(ヘッドレスの Chrome で、合成の入力の揺れの目安は平滑化なしで約 3.3、既定の係数で約 0.55、回転の `beta: 0` で約 0.17)。

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

### 届いた値で動かす (受け手)

別の場所 (配信のスタジオの指揮役など) で作ったパラメータの値を受け取って描くときは、`update` の代わりに `follow` を使います (T0133・T0134・T0135)。

```js
// values: Float32Array (puppet.parameter_ids() の順。指揮役の物理の前の値)。dt: 秒。lipSync: [音量, 強さ] か undefined
// parts: Uint32Array (モーションのパーツの不透明度のカーブがあるパーツの番号)。opacities: Float32Array (値と重みを交互に)
for (const { values, dt, lipSync, parts, opacities } of frames)
  puppet.follow(values, dt, lipSync?.[0], lipSync?.[1], parts, opacities); // 届いた順にすべて適用する (飛ばさない)
puppet.render(); // 描くのは最後だけでよい
```

- `follow` は値を **基準値として** 書き、物理 → リップシンク (渡した音量と強さで、このモデルの対象に当てる) → パーツの不透明度
  (渡したモーションのカーブとポーズ) の順に進めます。
  このパペットのモーション・表情・トラッキング・目パチ・呼吸と、`set_lip_sync_level` の音量は使いません。
  同じ列なら、どのページ・ネイティブの `Puppet::follow` でも物理を含めて同じ形になります (ネイティブとは許容誤差内)。
- 値をまとめて読む・書くには `parameter_values()` / `set_base_values(values)` (どちらも `Float32Array`) を使います。
  `set_parameter` と違い、`set_base_values` は前のフレームの出力と同じ値でも基準値にします。
- 数が `parameter_ids()` と違う・NaN を含むときは、何も変えずに例外を投げます。

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

`<ugoe-avatar>` (属性・スライダーからの `setParameter`・表情とモーションの切り替え・`backend` の切り替え・合成の入力とカメラのトラッキング) と、
`createPuppet` で canvas に直接描く例を並べています。モデルはリポジトリの `assets/samples/` (MIT) を配ります。
URL の `?src=` で `<ugoe-avatar>` のモデルを、`?simulate=` で WebGPU の失敗を ([上](#webgpu-と-webgl2)) 変えられます。

見本のモデル (`avatar.ugoe`・`avatar-lit.ugoe`・`dress-up/dress-up.ugoe`) はモーション (待機の `idle`・うなずく `nod`・
首を振る `shake`・首をかしげる `tilt`) と表情 (`avatar.ugoe` は `smile`・`surprised`・`sad`・`angry`、`dress-up.ugoe` は
`smile`・`surprised`・`sad`) を持つので、`motion` / `expression` 属性をそのまま試せます
([assets/samples/README.md](../../assets/samples/README.md#モーションと表情)。T0719)。

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
| 前 (`just wasm` と同じ: release、lto = "thin"。T0410 の前) | 7,920,320 | 1,974,207 | 1,310,795 |
| **いま** (T0410: 下のプロファイルと名前の節を外す) | **4,455,303** | **1,301,592** | **964,869** |

(バイト。gzip と brotli は Node の zlib で測った。「いま」は T0413 の時点の main で測り直した値)

- cargo のプロファイル `web-dist` を `--config` で定義して使います: `opt-level = "z"`・`lto = "fat"`・`codegen-units = 1`。
  ただし毎フレーム動くクレート (`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe-render`・`ugoe`・`ugoe-core`・`glam`) は
  `opt-level = 3` に戻します。すべてを `"z"` にすると 4,354,077 バイト (gzip 1,250,656。T0410 の時点) とさらに小さいものの、
  CPU の評価 (`update` と変形) が約 2.5 倍遅くなったためです (見本のアバターで 1 フレーム約 19µs → 48µs。ヘッドレスの Chrome)。
- wasm-bindgen の `--remove-name-section` で関数名の節 (約 1.8MB。パニックのスタックトレースに名前が出るだけ) を外します。
- **wasm-opt (binaryen) は使いません。** `-Oz` で生の大きさは約 1 割減りますが、gzip した大きさはかえって約 1 割増えました。
- **MessagePack の読み込みは `rmp_serde` のまま (T0413 で試して入れなかった)**: `rmp_serde` の読み込みは型ごと (visitor ごと) に
  マーカーの解釈まで展開され、約 1.15MB (モデルの型の `Deserialize` を含む) を占めている。T0413 で、読み込みの結果を `rmp_serde` と同じに保つ
  自前の `Deserializer` を ugoe-format に書いて試すと、約 0.63MB になり全体で 508KB (生で −11%) 減ったが、繰り返しの多いコードだったので
  gzip では 32KB (−2.4%)、brotli では 17KB (−1.7%) しか減らなかった。読み込みはネイティブで約 0.39ms → 約 0.46ms (見本のアバターの `MODL` 122KB)。
  外から来る入力を読む部分を自前で持つ負担に対して、転送の大きさの効果が小さいので入れなかった (利用者の判断)。
- 残りの大部分は MessagePack の読み込み (約 1.15MB)・naga (シェーダの変換、約 0.7MB)・wgpu (約 0.77MB)・
  `serde_json` (約 0.37MB。表情・モーション・エフェクトなどの JSON の読み書き) です。
- **`serde_json` は今のまま (T0719 で見直して変えなかった)**: twiggy で見ると `serde_json` の関数は約 414KB (名前を残したビルドで数えた値) で、
  ほとんどがモデルの型の `Deserialize` を JSON 用に展開したもの (`StrRead` の読み込み約 240KB、`#[serde(tag)]` などの enum の
  中間の値など約 135KB)。その約半分 (約 200KB) はモデルの JSON (`load_json`・`loadModel` に文字列を渡す口。エディタのビューポートも使う) の読み込みで、
  残りはモーション・表情・物理・エフェクト・ライティングの JSON (どれも公開の API)。読み込みの口をすべて残したまま型の展開を減らすには、
  JSON を別の形 (MessagePack など) に写してから読む・自前の読み込みを書く、のどちらかが要り、エラーの文 (行と列) などの結果が変わる。
  上限を知るため、モデルの JSON の読み込みを丸ごと外したビルドを測ると、4,475,706 → 4,289,977 (生で −4.2%)、
  gzip 1,311,151 → 1,270,981 (**−3.1%**)、brotli 969,758 → 946,831 (−2.4%) だった (API を壊すので入れていない)。
  どの方法でもこれより減らないうえ、T0413 の MessagePack と同じく圧縮後の効果が小さいので、変えないことにした。
  ほかに、`serde_json::from_slice` (バンドルの `META`) と `from_str` で読み込みが 2 通りに展開されている分は約 13KB で、揃えても効果は無い。

### エディタとビューアの wasm は今のまま (T0413)

エディタとビューアの wasm (`just wasm`。release・lto = "thin"・関数名の節あり) には、このプロファイルを使わないことにしました。

| T0413 の時点 | 大きさ | gzip (-9) | brotli |
| --- | --- | --- | --- |
| `just wasm` (release、名前あり) | 7,922,205 | 1,975,421 | 1,312,668 |
| release、名前なし | 6,092,526 | 1,760,368 | 1,194,868 |
| `web-dist`、名前あり | 8,694,495 | 1,732,401 | 1,170,533 |
| `web-dist`、名前なし (このパッケージ) | 4,455,303 | 1,301,592 | 964,869 |

- 両方ともアプリ (Tauri) の中の wasm で、ネットワークを通らない。小さくして効くのはコンパイルの時間だが、
  V8 (Node 26) の `WebAssembly.compile` は release で約 5.9ms、`web-dist` で約 4.9ms と、どちらも小さい。
- 関数名の節は、開発中のパニックのスタックトレースに要る。名前を残すと `web-dist` でも release より大きい (名前が長くなる)。
- `lto = "fat"`・`codegen-units = 1` は、`ugoe-runtime` を触った後の作り直しが約 4.1 秒 → 約 7.3 秒と遅くなる (`just wasm` は開発中に何度も回す)。
- 評価の速さは変わらない (release はすべて `opt-level = 3`。`web-dist` も毎フレーム動くクレートは 3)。
- ビューアをブラウザ版として Web に置くようになったら、そのビルドには `web-dist` (`scripts/build.mjs` の `PROFILE_CONFIG`・`HOT_CRATES`) を使う。

## 開発

| コマンド | 内容 |
| --- | --- |
| `pnpm --filter @ugoe/runtime build` | = `just web-runtime`。wasm のビルド (`src/wasm/` に生成、gitignore 済み)・`tsc`・`dist/` への出力 |
| `pnpm --filter @ugoe/runtime check` | 型検査。`src/wasm/` に生成物 (`ugoe_wasm_bg.wasm`) が無ければ、手書きの型 `stub/ugoe_wasm.d.ts` を `src/wasm/` に写し直して検査する (写しは毎回作り直すので、スタブを直せばそのまま反映される) |
| `pnpm --filter @ugoe/runtime test` | 属性の解釈などの単体テスト (Vitest) |

- `crates/ugoe-wasm` の公開 API を変えたら、`stub/ugoe_wasm.d.ts` を生成物 (`src/wasm/ugoe_wasm.d.ts`) から作り直します。
- このパッケージは MIT です。`apps/`・`packages/` (GPL-3.0-only) のコードを import したり写したりしないでください ([ADR 0025](../../docs/adr/0025-web-runtime-package.md))。
- 構成: `src/index.ts` (入口)・`src/runtime.ts` (初期化・作成・読み込み)・`src/avatar.ts` (`<ugoe-avatar>`)・
  `src/element.ts` (登録するだけの入口)・`src/attributes.ts` (属性の解釈。DOM と wasm に触れない)。
