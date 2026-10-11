# @ugoe/runtime

ugoe のランタイムを Web で使うための npm パッケージです。中身は次の 3 つです。

- ugoe-wasm (`crates/ugoe-wasm`) の wasm と JS バインディング (`WebPuppet`) と、その TypeScript の型
- 初期化・作成・読み込みの薄い補助 (`initRuntime`・`createPuppet`・`loadModel`・`fetchModel`。読み込みの上限 `LoadLimits`)
- Web Component の `<ugoe-avatar>`

WebGPU が使えれば WebGPU、使えなければ WebGL2 で描きます。ライセンスは **MIT** です
([ADR 0025](../../docs/adr/0025-web-runtime-package.md)。GPL-3.0-only の `packages/` とは別の場所に置いています)。
wasm に入る Rust のクレートのライセンスの表記は `THIRD-PARTY-NOTICES.txt` (パッケージに入る。生成と検査は
[docs/release.md#mit-の配布物](../../docs/release.md#mit-の配布物))。

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
| `power-preference` | `default` (既定。希望なし) / `low-power` / `high-performance`。GPU の電力の希望 (下の「[GPU の電力の希望](#gpu-の電力の希望)」)。作るときと、GPU を失って作り直すときに渡す。変えると作り直す (`backend` と同じ。意味の変わらない書き換えでは作り直さない)。知らない値は `default` として扱い、警告 (`unknown power-preference`) を出す。プロパティは `powerPreference` |
| `parameters` | `"ID: 値; ID: 値"` (区切りは `;` か改行、`:` の代わりに `=` も可)。読み込みのたびと、属性を変えたときに設定する。属性から消した ID は元に戻さない |
| `expression` | バンドルの表情の名前。消すと表情を止める |
| `motion` | バンドルのモーションの名前。消すとモーションを止める。再生中のモーションより優先度が低くて始められなければ、始められるまで毎フレーム試す (待機モーションは、ほかのモーションが終わってから流れる) |
| `motion-priority` | `idle` (既定) / `normal` / `force` |
| `motion-over-tracking` | 付けると、トラッキング中も、再生中のモーションが動かすパラメータではモーションを優先する (下の「[フェイストラッキング](#フェイストラッキング)」。T0722) |
| `tracking-kept` | `motion-over-tracking` でもトラッキングのままにするパラメータの ID (`"EyeLOpen EyeROpen"`。区切りは空白・`,`・`;`) |
| `paused` | 時間を止める (物理・モーション・目パチ・呼吸を進めない)。パラメータを変えたときだけ描き直す |
| `crossorigin` | `src` の取得の資格情報 (`<img crossorigin>` と同じ対応)。`use-credentials` なら別のオリジンにも Cookie などを送る (`credentials: "include"`)。省略・`anonymous` は同じオリジンにだけ送る (`fetch` の既定)。次の取得から効く (変えただけでは取り直さない)。プロパティは `crossOrigin` |

パラメータの ID は大文字と小文字を区別します (HTML の属性名は小文字になるので、パラメータごとの属性にはしていません)。

```ts
import "@ugoe/runtime/element";

const avatar = document.querySelector("ugoe-avatar")!; // UgoeAvatarElement (HTMLElementTagNameMap に登録済み)
avatar.addEventListener("load", (e) => {
  // e は CustomEvent<UgoeAvatarLoadDetail>
  console.log(e.detail.backend, e.detail.warnings); // warnings: 文の警告 (英語)
  for (const w of e.detail.warningDetails) console.log(w.code); // 同じ警告の構造 (下の「読み込みの警告」)
});
avatar.addEventListener("error", (e) => {
  // e は CustomEvent<UgoeAvatarErrorDetail> (HTMLElement の ErrorEvent ではない)
  console.error(e.detail.error);
});

avatar.setParameter("AngleX", -20);       // 値域にクランプ。ID が無ければ false
avatar.parameters = { AngleY: 10, BodyAngleX: 5 };
avatar.parameters;                         // { AngleX: -20, AngleY: 10, ... } (今の値)
avatar.parameterDefinitions;               // [{ id, name, min, max, default, ... }]
await avatar.load(bytesOrJsonString);      // src の代わりに直接読み込む。戻り値は文の警告
avatar.warningDetails;                     // 最後の読み込みの警告の構造 (avatar.warnings と同じ順)
avatar.powerPreference = "low-power";      // power-preference 属性を書き換える (作り直す)
avatar.adapterName;                        // 使っている GPU の名前。作成前と切断中は null、ブラウザが伏せると ""
const puppet = await avatar.ready;         // WebPuppet そのもの (下)。作成に失敗したら reject (下の「失敗とやり直し」)
puppet.set_lighting_json(JSON.stringify({ ambient: [0.5, 0.5, 0.5] }));
avatar.requestRender();                    // puppet を直接操作したら描き直しを頼む
avatar.setTracking({ blendshapes: { jawOpen: 0.4 }, head: { yaw: 10, pitch: 0, roll: 0 } }); // 下の「フェイストラッキング」
avatar.clearTracking();
```

- 大きさは要素の CSS の大きさ (既定 300x300) × `devicePixelRatio` です。背景は透明なので、要素の `background` がそのまま見えます。
  `devicePixelRatio` だけが変わったとき (倍率の違う画面へウィンドウを移したなど。ResizeObserver は知らせない) も、
  `matchMedia` で聞いて描画バッファの大きさを合わせ直します。
- 画面外にあるとき (IntersectionObserver) と `paused` の間は `requestAnimationFrame` を回しません。
  回している間も、形と見せ方が変わらないフレームは描きません ([描画のループ](#描画のループ))。
- GPU を失ったら (GPU プロセスのクラッシュなど)、新しい canvas で作り直してモデルを読み直します (パラメータの値は引き継ぐ)。
  WebGL2 では、canvas の `webglcontextlost` で描くのを止め、`webglcontextrestored` を待ってから作り直します
  (`puppet.is_lost()` も `true` になりますが、復元を待つために `preventDefault` するので、要素も canvas のイベントを聞きます)。
  復元が 5 秒待っても届かなければ、新しい canvas で作り直しを試します。作り直しで作れなければ (GPU がまだ戻っていない)、
  5 秒ごとに試し直します。前の作り直しから 1 分以内に失う・作れないことが続き、4 回目になったら、作り直さずに `error` を送ります。
- GPU を失って戻るのを待ち始めたら (WebGL2 の復元を待つ・作り直しを試し直すのを待つ)、`gpulost` を送り、`waitingForGpu` が
  `true` になります。作り直せたら (`ready` が解決したら) と諦めたら (`error`) `false` に戻ります。待つ間に
  「GPU の復元を待っています」などを出すのに使います。

  ```ts
  avatar.addEventListener("gpulost", async () => {
    showMessage("GPU の復元を待っています");
    try {
      await avatar.ready; // 作り直せた
      hideMessage();
    } catch {
      showRetryButton(() => avatar.restart()); // 続けて失って諦めた
    }
  });
  ```
- ページから外すと GPU の資源を捨て、戻すと作り直します。取得中の `src` は外しても続け、戻したらその結果を読み込みます。
  `src` を変えたり `load()` を呼んだりして置き換わった取得は中断します (`AbortController`)。
- 読み込めなかったモデル (`load` の reject・`src` の `error`) は覚えず、作り直したときは最後に読み込めたモデルを読み直します。
- 読み込みの途中に GPU を失ったとき (`load_bundle` などが `LoadError` の `device_lost` を投げたとき) は、読み込めなかったとは
  しません。`error` を送らず、`load()` も reject せずに待ち、上の作り直しの後にそのモデルを読みます (`load()` はそこで解決し、
  `load` を送ります)。`src` の取得が失っている間に終わったときも、作り直した後にその `src` を読みます。別のモデルなので、
  前のモデルのパラメータの値は引き継ぎません。作り直しを諦めたときだけ、`error` を送って `load()` を reject します。
- 失敗とやり直し:
  - `WebPuppet` の作成に失敗したとき (`auto` なら WebGL2 でも失敗したとき) と、GPU を続けて失って作り直しを諦めたとき
    (`the GPU device was lost repeatedly`) は、`error` を送り、`ready` をその理由で reject します。
    誰も `ready` を待っていなくても unhandledrejection にはなりません。
  - その状態では、作成 (作り直し) を待つ `load()` も reject し、後から呼んだ `load()` も待たずに reject します。`src` などの属性は覚えておきます。
  - やり直すには `avatar.restart()` を呼ぶか、`backend`・`power-preference` を変えるか、ページから外して戻します。最後に読み込めたモデル
    (取得中の `src` があればその結果) を読み直し、パラメータの値は引き継ぎます。`restart()` は作り直しの回数も数え直します。
  - 作り直しを始めると (`gpulost` を送ったときも)、`ready` は新しい `WebPuppet` を待つ Promise に変わります (`await avatar.ready` は作り直すたびに取り直す)。

  ```ts
  try {
    await avatar.ready;
  } catch (e) {
    showRetryButton(() => avatar.restart()); // GPU が戻ったら利用者に押してもらう、など
  }
  ```

- 別の名前で登録するなら、`@ugoe/runtime/element` の代わりに `defineUgoeAvatar("my-avatar")` を呼びます。
- イベントの型は `UgoeAvatarElementEventMap` (`load`: `CustomEvent<UgoeAvatarLoadDetail>`、`error`: `CustomEvent<UgoeAvatarErrorDetail>`、
  `gpulost`: `CustomEvent<null>`)。
  `UgoeAvatarElement` の `addEventListener` / `removeEventListener` はこれで `e.detail` を型付けします
  (要素を `HTMLElement` として持っているときは `UgoeAvatarElement` にしてから呼びます)。

### 描画のループ

`<ugoe-avatar>` は、時間を進めている間 (`paused` でなく画面に見えている間) は `requestAnimationFrame` のたびに `update(dt)` を呼び、
次のどちらかのときだけ `render()` を呼びます (T1402)。

- 形が変わった: `puppet.shape_changed()` が `true` (前の `render` の後に、評価の出力が見て分かるほど変わった。パラメータの書き換え・
  モーション・表情・目パチ・呼吸・物理・トラッキング・読み込みを含む)。
- 見せ方が変わった・描き直しを頼まれた: `requestRender()` の後。要素は大きさ (`devicePixelRatio` を含む)・属性
  (`parameters`・`expression`・`motion`・`paused` など)・`setParameter`・`setTracking`・`clearTracking`・読み込み・GPU の作り直しの
  たびに自分で呼びます。

待機のモーション・目パチ・呼吸の無いモデル (や、それらを止めたページ) は、読み込んで 1 回描いた後は描きません。物理は揺れている間は
描き、揺れが見て分からない大きさに収まると描かなくなります。描かない間も `update` で時間 (物理) は進むので、動き出したフレームで
すぐ描きます。WebGPU・WebGL2 の canvas は描かなければ前の画を出し続けるので、見た目は変わりません。

決めたこと:

- **「形が変わらない」は SDK (`Puppet::changed`) で決める。** wasm の `shape_changed` はそれを `render` まで積んだもの。要素の中で
  パラメータの値を毎フレーム読んで比べる案もあったが、判断を下の層に置くと C ABI・Unity・Godot のホストも同じ口で描くのを省け、
  比べるのも `Puppet` の中で安い (境界を越えて値を読まない)。パラメータでなく評価の出力 (頂点位置・不透明度・色・描画順) で比べるので、
  描画に効かないパラメータの変化では描かず、動的物理 (パラメータを持たない揺れ) や着せ替えも同じ規則で扱える。
- **許容誤差** は頂点位置がキャンバスの長い辺の 1e-5 (4K の画面でも 0.04 画素)、不透明度と色が 1e-3 (8 ビットの 1 段の 4 分の 1)。
  比べる相手は最後に描いた出力なので、ゆっくりした変化も積もれば描く。決め方と理由は
  [docs/spec/evaluation.md#見た目の変化](../../docs/spec/evaluation.md#見た目の変化)。
- **見せ方の変化は要素が知っている。** `puppet` を直接操作して見せ方を変えた (ビュー・背景・光源・テクスチャ・着せ替えなど) ときは、
  今までどおり `requestRender()` を呼びます。パラメータを直接書いたとき (`puppet.set_parameter`) は `shape_changed()` が `true` に
  なるので描きますが、呼んでも構いません。
- **描けなかったフレームは次のフレームで描き直す (T0983)。** `render()` は描けたかを返し、サーフェスを作り直したときや時間切れの
  ときは描かずに `false` を返します。要素はそのとき描き直しの印を残し、次のフレームでもう一度 `render()` を呼びます
  (形の変化は `shape_changed()` が `true` のまま残る)。自分のループで描くときも、見せ方の変化の印は `render()` が `true` を
  返してから下ろします。
- 利用者の操作は要りません (描く回数が減るだけ)。
- canvas を `drawImage`・`toDataURL` で写すときは注意します。描画バッファは描いた直後 (同じタスク) にしか読めず
  (`preserveDrawingBuffer: false`)、描かなかったフレームの後に写すと空になります。画を取り出すなら `puppet.read_pixels()` を使うか、
  `requestRender()` の後に要素が描いた直後に写します (適合テストの `tests/conformance/web/avatar.js` は `render` を包んで写す)。

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

### MediaPipe の利用状況の送信

MediaPipe (`@mediapipe/tasks-vision`。1.0.1 で確かめた) は、映像や画像をこの計算機の中で処理しますが、利用状況 (タスクの種類・
実行モード・OS・版・処理時間の統計。映像やランドマークは含まない) を、Landmarker を作ってから 60 秒ごとと `close()` したときに
`https://odml.pa.googleapis.com/v1/log` へ `fetch` で送ります。止める設定はありません
([MediaPipe Tasks のプライバシーの説明](https://goo.gle/mediapipe-privacy)。送るなら利用者の同意を得るのはアプリの責任とされています)。
このパッケージは MediaPipe に依存しないので、送るかどうかはホストが決めます。送らせないなら次のどちらか (両方でもよい) を使います。

- CSP の `connect-src` で送り先を許さない (MediaPipe の wasm とモデルを読む場所だけを許す)。送ろうとするたびに CSP の違反が報告されます。

  ```html
  <meta http-equiv="Content-Security-Policy"
        content="connect-src 'self' https://cdn.jsdelivr.net https://storage.googleapis.com">
  ```

- Landmarker を作る前に `fetch` を包み、送り先への要求をネットワークの失敗として断る。MediaPipe は送るたびにグローバルの `fetch` を
  呼び、1 回失敗すると以後は送りません。要求はネットワークに出ないので、CSP の違反も出ません。

  ```js
  const pageFetch = window.fetch.bind(window);
  window.fetch = (input, init) => {
    const url = new URL(input instanceof Request ? input.url : String(input), location.href);
    return url.hostname === "odml.pa.googleapis.com"
      ? Promise.reject(new TypeError("MediaPipe usage logging is disabled"))
      : pageFetch(input, init);
  };
  ```

ugoe のアプリ (エディタ・ビューア・スタジオ) は両方を使っています ([packages/face-tracking](../../packages/face-tracking/README.md#利用状況の送信-t0943))。
見本 (`examples/basic`) の「camera (MediaPipe)」も、Face Landmarker を作る前に上と同じように `fetch` を包むので送りません
(断った回数は見本のページの `window.ugoeExample.usageLogsRefused`)。送り先への要求がネットワークに出ないことは、適合テストの
`just conformance --only example` が、ヘッドレスの Chrome の偽のカメラで見本を開いて確かめます (CI の `conformance` ジョブ。
[tests/conformance/README.md](../../tests/conformance/README.md#見本の-mediapipe-の利用状況の送信))。

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
const puppet = await createPuppet(canvas, { backend: "auto" }); // wasm の初期化もする。powerPreference も渡せる (下)
const warnings = await fetchModel(puppet, "avatar.ugoe");        // または loadModel(puppet, bytes)
puppet.set_parameter("AngleX", 15);
let last = performance.now();
requestAnimationFrame(function frame(now) {
  if (puppet.is_lost()) return; // GPU を失った (WebGL2 のコンテキストの喪失を含む)。新しい canvas で作り直す
  puppet.update(Math.min((now - last) / 1000, 0.1));
  last = now;
  // 形が変わったときだけ描く。見せ方 (大きさ・ビュー・背景・光源) を変えたフレームは自分で描く (下の「描画のループ」)。
  // 描けなかった (`render()` が false) ときは shape_changed() が true のまま残り、次のフレームで描く
  if (puppet.shape_changed()) puppet.render();
  requestAnimationFrame(frame);
});
```

`WebPuppet` の API (物理・モーション・表情・トラッキング・ライティング・着せ替えなど) は
[crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md) を見てください。NaN・無限大・負の `dt` を渡したときの扱い
(`update` と `set_parameter` は例外を投げずに 0 や「書かない」として扱い、`follow` と `set_base_values` は例外) は
[crates/ugoe-wasm/README.md#有限でない値と-dt](../../crates/ugoe-wasm/README.md#有限でない値と-dt)、ほかの入口との比較は
[docs/portability.md#ホストから来る数](../../docs/portability.md#ホストから来る数)。

### 公開の API の範囲 (T0492)

`WebPuppet` のメソッドのうち、アプリ向けの問い合わせ (`deformer_ids`・`warp_points`・`rotation_frame`・
`rotation_local_keyform`・`deformer_point`・`deformer_jacobian`・`physics_particles`・`part_ids`) は **公開の API の外** です。
呼べて型にも出ますが、アプリ (エディタのギズモ・物理の画面とスタジオの受け手) の都合で予告なく変わり、版を上げるときの互換の対象にしません。
型では doc コメントの `@internal` で示します (一覧と理由は
[crates/ugoe-wasm/README.md#アプリ向けの問い合わせ-ギズモ物理の表示パーツの並び](../../crates/ugoe-wasm/README.md#アプリ向けの問い合わせ-ギズモ物理の表示パーツの並び))。
配布物から外さない (feature で分けない) のは、wasm と型の写しを 2 通り持つ手間に対して大きさの得が小さいためです。
どのメソッドもランタイムの評価の結果を読んで並べ直す薄い口で、外しても減るのはその分だけです (測ってはいない。全体の内訳は
[wasm の大きさ](#wasm-の大きさ))。

### 読み込みの警告と着せ替えの失敗の型 (T0438)

`load_warnings_json()` と着せ替えの `DressUpError` は、このパッケージの型と補助で読めます
(`code` と補足の値の一覧は [crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md#読み込みの警告-t0125) の表)。

```ts
import {
  detachedModules,
  fetchModel,
  isDressUpError,
  isKnownLoadWarning,
  loadWarnings,
  type UgoeLoadWarning,
} from "@ugoe/runtime";

function describe(w: UgoeLoadWarning): string {
  // 新しい版のランタイムは知らない code を返しうるので、先に絞る (絞らないと code で補足の値の型が決まらない)
  if (!isKnownLoadWarning(w)) return w.warning;
  switch (w.code) {
    case "texture_ktx2":
    case "texture_missing":
    case "normal_map_ktx2":
      return `テクスチャ ${w.texture} を代用しました`;
    case "physics_removed":
      // 理由が検査のエラーなら w.error_codes に種類 ("unknown_reference" など) が入る
      return "物理を外しました";
    case "module_detached":
      return `${w.module} を外しました (${w.error.code})`;
    case "unknown_optional_feature":
    case "unknown_value":
      return w.warning;
  }
}

const warnings = await fetchModel(puppet, "avatar.ugoe"); // 文の警告 (ログ用)
const details = loadWarnings(puppet);                     // 同じ順・同じ数の構造 (UgoeLoadWarning[])
detachedModules(puppet);                                  // 付け直せずに外したモジュールだけ (UgoeModuleDetachedWarning[])

try {
  puppet.attach_module("Hat");
} catch (e) {
  if (!isDressUpError(e)) throw e;
  // e.code: UgoeDressUpErrorCode ("slot_occupied" など。知らない値もありうる)
  // 補足の値 (無ければ undefined): e.module・e.slot・e.parameter・e.mesh・e.attached (slot_occupied)・e.mask (unknown_mask)・
  // e.rig・e.collider (unknown_collider)・e.part (pose_not_exposed)・e.texture (texture_out_of_range。数)・
  // e.required・e.actual (avatar_level。数)・e.errors (invalid_physics・invalid_composition の検査のエラーの数)・
  // e.error_codes (同じく検査のエラーの種類を ", " でつないだ文字列。T0168)
  if (e.code === "slot_occupied") console.warn(`${e.slot} には ${e.attached} が付いています`);
}
```

- `isKnownLoadWarning` で絞らずに `code` の文字列で比べることもできます (補足の値は `unknown` になる)。
- このパッケージが知っている `code` の一覧は、配列 `LOAD_WARNING_CODES`・`DRESS_UP_ERROR_CODES`・`LOAD_ERROR_CODES`・`CREATE_ERROR_CODES`・`RENDER_ERROR_CODES`・`SETTINGS_ERROR_CODES` でも読めます
  (訳の表が全種類を持つかのテストなどに。型 `UgoeDressUpErrorCode`・`UgoeLoadErrorCode`・`UgoeCreateErrorCode`・`UgoeRenderErrorCode`・`UgoeSettingsErrorCode` はこの配列の要素)。
  ランタイムの `code` との突き合わせは下の「開発」。
- `<ugoe-avatar>` の `load` イベントの `detail.warningDetails` と `avatar.warningDetails` は `loadWarnings(puppet)` と同じものです。
- `isDressUpError` は `name` が `"DressUpError"` で `code` が文字列の `Error` を受けます。`set_base_physics_json` で付けているモジュールと
  合成できない失敗も同じ `DressUpError` です (T0476)。`message` は英語の文で、安定した値ではありません。

### 読み込みの上限 (T0152)

置かれた環境 (メモリの少ない端末・外から来るファイルを開くページなど) に合わせて、読み込みの上限を小さくできます。
上限はファイル形式の決まりではなく、読む側の方針です (`docs/spec/file-formats.md#読み込みの上限`)。

```ts
import { fetchModel, isLimitError, type LoadLimits, limitsJson, loadModel } from "@ugoe/runtime";

const limits: LoadLimits = {
  maxChunkDecompressed: 64 * 1024 * 1024, // 圧縮したチャンク 1 つの展開後 (バイト)。既定 256 MiB
  maxTotalDecompressed: 256 * 1024 * 1024, // 1 つのファイルで展開するバイトの合計。既定 1 GiB
  maxImageSide: 4096, // デコードする PNG の幅と高さ (画素)。既定 16384
};
try {
  await fetchModel(puppet, "avatar.ugoe", undefined, { limits }); // loadModel(puppet, bytes, { limits }) も同じ
  puppet.load_module(modBytes, limitsJson(limits)); // 着せ替えのモジュールは WebPuppet に JSON で渡す
} catch (e) {
  if (!isLimitError(e)) throw e;
  // e.code: "read_limit_exceeded" (e.limit・e.chunk・e.max) | "image_limit_exceeded" (e.image・e.texture・e.width・e.height・e.max)
  console.warn(`too large: ${e.code}`);
}
```

- 書かなかった上限は既定値です。超えたときは何も変えません (前のモデルのまま)。`code` と補足の値の一覧は
  [crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md#読み込みの上限-t0152)。
- `<ugoe-avatar>` では `avatar.loadLimits = limits` (属性は無い)。次の読み込み (`src`・`load()`・作り直しの読み直し) から効き、
  超えたら `src` では `error` イベントの `detail.error`、`load()` では reject の理由が上の例外になります。
- `loadModel` の例外は `name` が `"LoadError"`、`load_module` は `"DressUpError"` です。`isLimitError` はどちらも受けます。
  上限の外の読み込みの失敗も `LoadError` で `code` を持ちます (下の「読み込みの失敗」)。
- JSON のモデル (`load_json`) は展開も画像のデコードもしないので、上限は効きません。

### 読み込みの失敗

`loadModel`・`fetchModel` (`load_json` / `load_bundle`) の失敗は、すべて `name` が `"LoadError"` で `code` と補足の値を持つ例外です
(T0434)。利用者に見せる文は `code` から作ります (`message` は英語の文で、安定した値ではありません)。

```js
import { isLoadError, loadModel } from "@ugoe/runtime";

try {
  loadModel(puppet, bytes);
} catch (e) {
  if (!isLoadError(e)) throw e;
  // e.code: "invalid_file" | "unsupported_version" (e.found・e.supported) | "unsupported_features" (e.features) |
  // "invalid_model" (e.errors・e.error_codes) | "compile" | "read_limit_exceeded" | "image_limit_exceeded" |
  // "invalid_texture" (e.image・e.texture) | "texture_too_large" (e.image・e.texture・e.width・e.height・e.max) |
  // "texture" | "device_lost"
  showError(e.code === "unsupported_version" ? "このファイルは新しい版で作られています" : e.message);
}
```

- 一覧と意味は [crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md#読み込みの失敗-t0434)。型は `UgoeLoadError`・`UgoeLoadErrorCode`。
  新しい版のランタイムは知らない `code` を返しうるので、知らないものは `message` を出してください。
- 失敗したときは何も変えません (前のモデルのまま)。

### 設定の差し替えの失敗 (T0461・T0469)

`WebPuppet` の `set_effects_json`・`set_physics_json`・`set_base_physics_json` は、渡した設定がモデルに対する検査を通らないと、`code` と
補足の値を持つ例外を投げます (何も変えません)。`name` は `"Error"` のままなので、`isSettingsError` で `code` を見て見分けます。

```ts
import { isSettingsError } from "@ugoe/runtime";

try {
  puppet.set_physics_json(json); // set_base_physics_json・set_effects_json も同じ
} catch (e) {
  if (!isSettingsError(e)) throw e; // 読めない JSON などは code を持たない Error
  // e.code: "invalid_physics" (set_physics_json・set_base_physics_json。着せ替えの invalid_physics と同じ) |
  //         "invalid_model" (set_effects_json。読み込みの invalid_model と同じ)
  // e.errors: 検査のエラーの数、e.error_codes: その種類を ", " でつないだ文字列 ("unknown_reference" など)
  console.warn(`physics rejected: ${e.error_codes}`);
}
```

| 関数 | `code` | 補足の値 |
| --- | --- | --- |
| `set_effects_json` | `invalid_model` | `errors`・`error_codes` |
| `set_physics_json`・`set_base_physics_json` | `invalid_physics` | `errors`・`error_codes` |

- 意味は [crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md#物理モーション表情エフェクト-t0309)。型は `UgoeSettingsError`・
  `UgoeSettingsErrorCode`、一覧の配列は `SETTINGS_ERROR_CODES` (crates/ugoe-wasm/codes.json の `settings_error` と同じ)。
- 読めない JSON は、`code` を持たない `Error` です。
- `set_base_physics_json` で付けているモジュールと合成できない失敗は、付け外しの失敗と同じ `DressUpError` です (`code` と補足の値も同じ。
  `isDressUpError` で見分ける。T0476)。
- `isSettingsError` は、`name` が `"Error"` で、このパッケージの知っている `code` を持つものだけを受けます (`name` が普通の `"Error"` なので。
  同じ `code` の `invalid_physics` を持つ `DressUpError` は受けません)。

### 作成の失敗 (T0475)

`createPuppet` (`WebPuppet.create`) は、GPU やブラウザの都合で作れないとき (canvas のコンテキストを取れない・取れた WebGL2 の
コンテキストが既に失われている・アダプタやデバイスを取れないなど) に、`name` が `"CreateError"` で `code` を持つ例外を投げます。
時間を置いて新しい canvas で作り直せば作れることがあります (`<ugoe-avatar>` は GPU を失った後の作り直しでこの失敗が起きたら、待って試し直し、
作り直しの回数に数えます。最初の作成なら `error` を送って `ready` を reject します)。

```ts
import { createPuppet, isCreateError } from "@ugoe/runtime";

try {
  const puppet = await createPuppet(canvas, { backend: "webgl2" });
} catch (e) {
  if (!isCreateError(e)) throw e;
  // e.code: "backend_unavailable" | "context_unavailable" | "adapter_unavailable" | "device_unavailable" (知らない値もありうる)
}
```

- 一覧と意味は [crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md#作成の失敗-t0475)。型は `UgoeCreateError`・`UgoeCreateErrorCode`、
  一覧の配列は `CREATE_ERROR_CODES`。知らない `backend` などの引数の誤りは `code` を持たない `Error` です。

### 描画と読み戻しの失敗 (T0465)

`WebPuppet` の `render()`・`read_pixels()` は、GPU を失った (`is_lost()`) 後と、読み戻しを待つ間に失ったときに、`name` が
`"RenderError"` で `code` が `"device_lost"` の例外を投げます (`read_pixels` は `Promise` の失敗)。新しい canvas で作り直してください
(`<ugoe-avatar>` は自分で作り直します)。

```ts
import { isRenderError } from "@ugoe/runtime";

try {
  const pixels = await puppet.read_pixels();
} catch (e) {
  if (!isRenderError(e)) throw e;
  // e.code: "device_lost" (知らない値もありうる)
}
```

- 一覧と意味は [crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md#描画と読み戻しの失敗-t0465)。型は `UgoeRenderError`・`UgoeRenderErrorCode`、
  一覧の配列は `RENDER_ERROR_CODES`。ほかの描画の失敗 (組み込みの誤りや一時的なもの) は `code` を持たない `Error` です。

### 問い合わせの JSON の型 (T0452)

`WebPuppet` の `*_json()` は JSON の文字列を返します。このパッケージの補助は `JSON.parse` して型を付けます
(形は [crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md) の各節)。

```ts
import {
  avatarSlots,
  currentPose,
  drawableStates,
  expressionInfos,
  moduleInfos,
  motionEvents,
  motionInfos,
  parameterDefinitions,
  sampleMotion,
  slotDefinitions,
  visiblePoseParts,
} from "@ugoe/runtime";

parameterDefinitions(puppet); // parameters_json():       { parameters: UgoeParameterDefinition[], groups: UgoeParameterGroup[] }
motionInfos(puppet);          // motions_json():          UgoeMotionInfo[] (name・duration・looping・fade_in・fade_out・parameters・events)
motionEvents(puppet);         // motion_events_json():    UgoeMotionEvent[] (直前の update のイベント。value・time と、発火したモーションの motion・number。T0462)
expressionInfos(puppet);      // expressions_json():      UgoeExpressionInfo[] (name・fade_in・fade_out・parameters: [{ parameter, value, op }])
moduleInfos(puppet);          // modules_json():          UgoeModuleInfo[] (warning_details は UgoeLoadWarning[])
slotDefinitions(puppet);      // slots_json():            UgoeSlot[] (空の一覧と無い値はキーごと無い)
avatarSlots(puppet);          // avatar_slots_json():     string[] | null (アバタープロファイルが無ければ null)
drawableStates(puppet);       // drawable_states_json():  UgoeDrawableState[] (drawable_ids() の順)
currentPose(puppet);          // pose_json():             UgoePose | undefined (モデルが無ければ undefined)
visiblePoseParts(puppet);     // pose_visible_json():     (string | null)[] | undefined (グループごとの表示中の要素のパーツ。T0478)
sampleMotion(motionJson, t);  // sample_motion_json():    UgoeCurveValue[] ({ target: { kind, id }, value })。wasm の初期化の後に呼ぶ
sampleMotion(sampler, t);     // MotionSampler.sample():  同じ形 (下の「モーションを何度も評価する」)
```

| `*_json()` | 型と補助 |
| --- | --- |
| `load_warnings_json`・`detached_modules_json` | `loadWarnings`・`detachedModules` (上の「読み込みの警告と着せ替えの失敗の型」) |
| `parameters_json`・`motions_json`・`motion_events_json`・`expressions_json`・`modules_json`・`slots_json`・`avatar_slots_json`・`drawable_states_json`・`pose_json`・`pose_visible_json`・`sample_motion_json` | 上の例 |
| `physics_json`・`base_physics_json` | **型を出さない。** 中身は `*.physics.json` の形 (リグ・粒子・入力・出力・衝突体などモデルの JSON 形式の大きな一部。[spec/schema/physics.schema.json](../../spec/schema/physics.schema.json)) で、主な使い道は保存して `set_physics_json`・`set_base_physics_json` に戻すこと。中を読むなら JSON Schema から型を作ってください |
| `MotionSampler.sample` | 形は `sample_motion_json` と同じ。`sampleMotion(sampler, t)` で `UgoeCurveValue[]` にする |

- 要素がモデルの JSON 形式と同じもの (パラメータ・スロット・ポーズ) は、新しい版のランタイムがキーを足しうります。知らないキーは無視してください。
- 新しい `*_json()` を足したら、ここに型と補助を足すか、この表に足さない理由を書きます。
- 型のキーは、本物の wasm の出力のキーと突き合わせています (下の「開発」の「問い合わせの JSON のキーの突き合わせ」)。

#### モーションを何度も評価する (`MotionSampler`、T0451・T0463)

`sampleMotion(json, t)` (`sample_motion_json`) は呼ぶたびに JSON を読み、カーブごとの索引を作ります。タイムラインの再生やシークのように
同じモーションを何度も評価するなら、`MotionSampler` を 1 度作って使い回します (区間を二分探索で引く。値は `sample_motion_json` と同じ)。
`MotionSampler` は ugoe-wasm のクラスそのもので、`WebPuppet` と同じく使い終えたら `free()` で解放します。

```ts
import { initRuntime, MotionSampler, sampleMotion } from "@ugoe/runtime";

await initRuntime();                          // new MotionSampler の前に wasm を初期化する
const sampler = new MotionSampler(motionJson); // *.motion.json の形。読めなければ例外
sampleMotion(sampler, 0.25);                  // UgoeCurveValue[] (フェードなし。値の無いカーブは含めない)
sampler.set_motion_json(editedJson);          // 編集したら差し替える (索引を作り直す。読めなければ例外で、前のモーションを保つ)
sampler.free();
```

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
- 指揮役になる側のパペットは、`update` の後に `values_before_physics()` (`Float32Array`)・`lip_sync_applied()`
  (`[音量, 強さ]` の `Float32Array`。当てていなければ `undefined`)・`part_opacities_applied_parts()` (`Uint32Array`)・
  `part_opacities_applied_values()` (`Float32Array`) で、上の `values`・`lipSync`・`parts`・`opacities` を読めます (T0454)。
  `parameter_values()` は物理の後の値なので、物理を持つモデルでは `follow` に渡しても指揮役と同じ形になりません。

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
描画に使っている API は `load` イベントの `detail.backend` か `puppet.backend()` で分かります。使っている GPU の名前は
`puppet.adapter_name()` (`<ugoe-avatar>` では `adapterName`) で分かります (ブラウザが伏せると空)。

### GPU の電力の希望

省電力の GPU (ノート PC の内蔵など) で描くか、高性能の GPU (外付けなど) を希望するかを選べます (T1401)。

```ts
const puppet = await createPuppet(canvas, { powerPreference: "high-performance" }); // "default" (省いたとき) | "low-power" | "high-performance"
puppet.adapter_name(); // 使っている GPU の名前
```

```html
<ugoe-avatar src="avatar.ugoe" power-preference="low-power"></ugoe-avatar>
```

- 名前は WebGPU の `requestAdapter` と WebGL の `getContext` の `powerPreference` と同じです。`createPuppet` は `WebPuppet.create` の
  3 つ目の引数に渡し、WebGPU ではアダプタを求めるとき、WebGL2 では canvas のコンテキストを作るときに使われます。
  `createPuppet` に知らない値を渡すと、コードを持たない `Error` です (`<ugoe-avatar>` の属性は `default` にして警告を出します)。
- 希望は作るときにしか効きません (WebGL2 は同じ canvas の 2 回目の `getContext` が属性を無視します)。そのため `<ugoe-avatar>` は
  `power-preference` を変えると、`backend` を変えたときと同じく新しい canvas で作り直します (モデルは読み直し、パラメータの値は引き継ぐ)。
  `auto` で WebGL2 に切り替えるときと、GPU を失って作り直すときも同じ値を渡します。
- **希望** なので、効くかはブラウザと OS が決めます (Windows の Chromium は WebGPU の希望を無視するなど)。効いたかは
  `adapterName` (`puppet.adapter_name()`) で確かめます。環境ごとの効き方は
  [crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md#gpu-の電力の希望-t0499)。

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

`<ugoe-avatar>` (属性・スライダーからの `setParameter`・表情とモーションの切り替え・`backend` と `power-preference` の切り替え・
合成の入力とカメラのトラッキング) と、`createPuppet` で canvas に直接描く例を並べています。モデルはリポジトリの `assets/samples/` (MIT) を配ります。
URL の `?src=` で `<ugoe-avatar>` のモデルを、`?simulate=` で WebGPU の失敗を ([上](#webgpu-と-webgl2))、`?power-preference=` で両方の例の
GPU の電力の希望 ([上](#gpu-の電力の希望)。選択肢に無い値は `default`) を変えられます。

「power-preference」の選択 (`default`・`low-power`・`high-performance`) を変えると `<ugoe-avatar>` は新しい canvas で作り直し、その下に
今の GPU の名前 (`adapterName`。ブラウザが伏せると「伏せています」) と描画の API (`webgpu`・`webgl2`) と希望を出します (作り直した後の
`load` で新しい値になります)。2 つの GPU を持つ機械で、希望によって GPU が変わるかを手で試せます (効くかはブラウザと OS が決めます)。

ヘッドレスのブラウザで確かめるために、見本はページの状態を `window.ugoeExample` に置きます (`version`・`<ugoe-avatar>` の `load` の結果の
`element` (`backend`・`powerPreference`・`adapterName` など)・低い層の例の `plain`・`errors`・トラッキングの `cameraFrames` や `usageLogsRefused` など)。`ready` は、操作の処理
(バックエンドの選択・チェックボックス・係数の欄) をすべて付け、`<ugoe-avatar>` を登録した後に `true` になる印です。自動のテスト
(`just conformance --only example`) はこれを待ってから操作します (モデルの読み込みは待たない。読み込みの完了は `element` で分かります)。

見本のモデル (`avatar.ugoe`・`avatar-lit.ugoe`・`dress-up/dress-up.ugoe`) はモーション (待機の `idle`・うなずく `nod`・
首を振る `shake`・首をかしげる `tilt`。`dress-up.ugoe` には手を振る `wave` (ポーズを切り替え、上げた手を揺らす。T0474・T0328) も) と表情 (`avatar.ugoe` は `smile`・`surprised`・`sad`・`angry`、`dress-up.ugoe` は
`smile`・`surprised`・`sad`) を持つので、`motion` / `expression` 属性をそのまま試せます
([assets/samples/README.md](../../assets/samples/README.md#モーションと表情)。T0719)。

### 計測のページ (`examples/timing`)

ブラウザの中で 1 フレームの評価 (`WebPuppet.update` の 1 回) を測るページです (T0192)。ネイティブの
`crates/ugoe/examples/puppet_timing.rs` と同じ 2 つのモデル (中程度のモデルを模した生成のモデルと見本の `avatar.ugoe`) を読み、
同じ入力 (トラッキング相当のパラメータの書き込み・リップシンクの音量・ループするモーションの再生) で `update`・受け手の `follow`・
物理の段を止めた `update` を測り、200 フレームずつの平均のうち最も速いものと中央値を表で出します。頂点の数・`navigator.userAgent`・
描画の API・タイマーの刻みも出します。測り方と結果は [docs/testing.md#1-フレームの評価](../../docs/testing.md#1-フレームの評価)。

リポジトリのルートで:

```sh
just web-runtime   # wasm と TypeScript をビルドして bindings/web/dist/ に出す
cargo run --release -p ugoe --example puppet_timing -- --write bindings/web/examples/timing/generated.ugoe
node bindings/web/examples/timing/measure.mjs --runs 5   # ヘッドレスの Chrome で開いて表を出す (CHROME で Chrome を指定)
node bindings/web/examples/timing/serve.mjs              # 手元のブラウザで開くなら http://localhost:8081/
```

- `serve.mjs` は `bindings/web` の `dist/` をそのまま配ります (tgz を作って入れなくてよい)。`performance.now()` の刻みを細かくするため
  (Chrome は 100µs → 5µs)、COOP と COEP を付けてページを cross-origin isolated にします。
- URL の `?frames=6000&batch=200&backend=webgl2` で、フレームの数・平均を取るフレームの数・描画の API を変えられます。
  V8 は wasm を後から最適化し直すので、フレームの数を減らすと遅く出ます。
- 結果はページの `window.ugoeTiming` (`done` が `true` になったら `environment` と `results` が揃う) にも置きます。
  `measure.mjs` はこれを読み、ページを `--runs` 回開き直した中央値と、測った前後の load average も出します。
- `measure.mjs --max-us 300` は `update` と `follow` の中央値を上限と比べ、超えたものを `OVER` と示して終了コード 1 で終わります
  (超えたらページを開き直して 2 回まで測り直す。判定は `check.mjs`、単体テストは `check.test.mjs`)。ビルドから比較までは `just perf-wasm`
  (`just perf` に含まれる。週に 1 度の CI の perf.yml も流す。[docs/testing.md#遅くなったら気づく検査](../../docs/testing.md#遅くなったら気づく検査))。

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
  ただし毎フレーム動くクレート (`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe-render`・`ugoe`・`ugoe-core`・`glam`・`libm`) は
  `opt-level = 3` に戻します (`libm` は T0193 で足した。物理が毎フレーム呼ぶ三角関数が約 1.5 倍速くなり、wasm は約 4KB 増えるだけ)。すべてを `"z"` にすると 4,354,077 バイト (gzip 1,250,656。T0410 の時点) とさらに小さいものの、
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
| `pnpm --filter @ugoe/runtime build` | = `just web-runtime`。wasm のビルド (`src/wasm/` に生成、gitignore 済み。隣に入力の指紋 `ugoe_wasm.fingerprint` を書く)・`tsc`・`dist/` への出力 (指紋は写さない)。`--wasm-only` で wasm のビルドだけ、`--no-wasm` で `src/wasm/` をそのまま使う |
| `pnpm --filter @ugoe/runtime check` | 型検査。`src/wasm/` に生成物 (`ugoe_wasm_bg.wasm`) が無ければ、手書きの型 `stub/ugoe_wasm.d.ts` を `src/wasm/` に写し直して検査する (写しは毎回作り直すので、スタブを直せばそのまま反映される)。生成物が今の ugoe-wasm より古ければ警告を出す (`just web-check` は先に `just wasm-fresh` で作り直す) |
| `pnpm --filter @ugoe/runtime test` | 属性の解釈・トラッキングの変換と平滑化・`<ugoe-avatar>` の作成と読み込みの順序 (happy-dom と `WebPuppet` の偽物)・問い合わせの JSON の補助・スタブの型のずれ・`code` の一覧と Rust の突き合わせの単体テスト (Vitest) |
| `just conformance --only wasm,wasm-webgpu,avatar,queries` | 配布物の適合テスト (`tests/conformance/README.md`)。`WebPuppet` の評価の結果と画像を参照と比べ (`wasm`・`wasm-webgpu`)、`<ugoe-avatar>` の作成・切り離しと再接続・`backend` の変更・WebGL2 のコンテキストの喪失からの作り直し (復元の時間切れ・作れないときの試し直しと諦めも) を、本物の wasm とヘッドレスの Chrome で確かめる (`avatar`)。問い合わせの JSON の出力のキーを型のキーと比べる (`queries`)。Chrome と wasm-bindgen-cli が要る |

- `crates/ugoe-wasm` の公開 API を変えたら、`stub/ugoe_wasm.d.ts` を生成物 (`src/wasm/ugoe_wasm.d.ts`) から作り直します。
  メソッドと関数の名前がクレートとずれていると、単体テスト (`stub/ugoe_wasm.test.mjs`) が失敗します。
  引数と戻り値の型まで含めたずれは、`just wasm` の後の `just wasm-types` (`scripts/check-wasm-types.mjs`。CI の web ジョブ) が報告します。
- 一度ビルドした `src/wasm/` は、pull などで ugoe-wasm が変わると古くなります (古い型で検査が落ちる)。`just wasm-fresh` は
  アプリの生成物と同じく指紋で古さを確かめ、古ければ `scripts/build.mjs --wasm-only` で作り直します
  ([docs/testing.md#wasm-の生成物の古さ](../../docs/testing.md#wasm-の生成物の古さ)。T0082)。
- このスタブは、生成物の無い環境でのアプリ (エディタ・ビューア・スタジオ) の型検査でも使います (各アプリの `tsconfig.json` の `paths`)。
  型の宣言をアプリが読むだけで、このパッケージが GPL のコードを読むわけではありません。
- **`code` の突き合わせ (T0452):** 読み込みの警告・着せ替えの失敗・読み込みの失敗・作成の失敗 (T0475)・描画と読み戻しの失敗 (T0465)・設定の差し替えの失敗 (T0469) の `code` と補足の値のキーの一覧は
  `crates/ugoe-wasm/codes.json` にあります。ugoe-wasm のテスト (`src/codes.rs`。`just ci` の Rust のテスト) がすべての種類の見本から作った一覧と、
  このパッケージのテスト (`src/codes.test.ts`。`just ci` の `web-check`) が `LOAD_WARNING_CODES`・`DRESS_UP_ERROR_CODES`・`LOAD_ERROR_CODES`・`CREATE_ERROR_CODES`・`RENDER_ERROR_CODES`・`SETTINGS_ERROR_CODES` と
  補足の値の型 (`UgoeDressUpErrorFields`・`UgoeLoadErrorFields`・`UgoeSettingsErrorFields`・`UgoeKnownLoadWarning` の各型) と比べます。Rust で種類やキーを足したら
  `UGOE_UPDATE_CODES=1 cargo test -p ugoe-wasm codes` で `codes.json` を書き直し、`src/warnings.ts`・`src/limits.ts`・`src/create-errors.ts`・`src/render-errors.ts`・`src/settings-errors.ts` も同じ PR で直します
  (片方だけでは、どちらかのテストが落ちる)。
- **問い合わせの JSON のキーの突き合わせ (T0463):** `src/queries.ts` の型ごとのキー (必須と省略できるもの) の一覧を
  `src/query-keys.ts` の `QUERY_KEYS` に値で持ちます (入口からは出さない)。一覧と型が同じことは `src/query-keys.test.ts` が
  `expectTypeOf` で確かめ (`pnpm check`)、本物の wasm の出力と一覧を比べるのは適合テストの `queries`
  (`just conformance --only queries`。`tests/conformance/web/queries.js`) です。見本のモデル (ポーズとパラメータのグループも持つ。T0468) を読み、すべての `*_json()`
  (警告と物理を除く) と `sample_motion_json`・`MotionSampler.sample` について、無い必須のキーと一覧に無いキーで落ち、1 度も出なかった形があっても落ちます。
  型にキーを足したら一覧にも足し (足さないと `pnpm check` が落ちる)、ugoe-wasm の出力にキーを足したら型と一覧に足します (足さないと `queries` が落ちる)。
- このパッケージは MIT です。`apps/`・`packages/` (GPL-3.0-only) のコードを import したり写したりしないでください ([ADR 0025](../../docs/adr/0025-web-runtime-package.md))。
- 構成: `src/index.ts` (入口)・`src/runtime.ts` (初期化・作成・読み込み)・`src/avatar.ts` (`<ugoe-avatar>`)・
  `src/queries.ts` (問い合わせの JSON の型)・`src/query-keys.ts` (その型のキーの一覧。適合テストが使う)・`src/warnings.ts`・`src/limits.ts` (警告と例外の型)・
  `src/element.ts` (登録するだけの入口)・`src/attributes.ts` (属性の解釈。DOM と wasm に触れない)。
