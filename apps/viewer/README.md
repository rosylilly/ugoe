# apps/viewer

ugoe のビューア (Tauri v2 + Svelte 5)。`.ugoe` バンドルか JSON のモデル (`model.json`) を開いて表示し、
パラメータを動かして確かめるだけの軽いアプリ。ランタイムをアプリに組み込むときの見本と、
フェイストラッキング (T0703) の確認の場を兼ねる。

**ライセンスは GPL-3.0-only** (エディタと同じ。[ADR 0019](../../docs/adr/0019-viewer-license.md))。
エディタのコードは写してよいが、アプリ同士で import はしない。

```
src/                  フロントエンド (Svelte 5 + TS。npm パッケージ @ugoe/viewer)
  App.svelte            ツールバー・ビューポート・右のパネル・ステータスバー、ドロップとショートカット
  lib/player.svelte.ts  WebPuppet (ugoe-wasm) を持ち、フレームのループを回す。UI の状態もここ
  lib/drivers.ts        パラメータを毎フレーム外から動かす仕組み (ParameterDriver)
  lib/host.ts           ファイルの選択と読み込み (Tauri / ブラウザ / 差し替え)
  lib/Viewport.svelte   描画領域 (ズーム・パン・フィット)
  lib/ParameterPanel.svelte  パラメータのスライダー (グループごと)
  lib/wasm/             ugoe-wasm の読み込み。generated/ は `pnpm wasm` の出力 (gitignore 済み)
public/               そのまま配る静的ファイル。third-party-notices.txt (同梱物のライセンス) だけコミットする
  mediapipe/            MediaPipe の wasm とモデル。`pnpm mediapipe` が用意する (gitignore 済み)
src-tauri/            バックエンド (Rust クレート `ugoe-viewer`)
```

フェイストラッキング (T0703) の部品 (MediaPipe の結果の変換・キャリブレーション・平滑化・記録・`FaceTracker`・
`TrackingPanel`) と、MediaPipe の wasm を写しモデルを取得して SHA-256 を検証するスクリプト (`ugoe-mediapipe-assets`) は、
エディタのプレビューと共有するパッケージ `@ugoe/face-tracking` ([packages/face-tracking/](../../packages/face-tracking/)) にある (T0636)。
ビューアは `Player` を `TrackingTarget` として渡し、文言は日本語 (`ja`)、記録の保存は `host.ts` の `saveJsonFile`。

## 仕組み

- 描画もパラメータの操作も、webview の中の `ugoe-wasm` (`WebPuppet`) で行う (エディタのビューポートと同じ、
  [ADR 0003](../../docs/adr/0003-editor-viewport-in-webview.md))。WebGPU が無ければ WebGL2 で描く。
- バックエンドは webview だけではできないことに限る: モデルのファイルをバイト列で読む (`read_model_file`) こと、
  トラッキングの記録を JSON で書く (`write_json_file`。保存のダイアログで選んだ `.json` だけ) こと、
  起動時の引数のパスを返す (`initial_file`) こと、webview からのカメラの許可の要求に応じること。依存してよいのはランタイム層のクレートだけ (`scripts/check-layers.mjs`)。
  いまは内部のクレートに依存していない。
- 形式は中身で判別する (先頭が `UGOE` ならバンドル、`{` なら JSON)。
- 見本のモデルはリポジトリの `assets/samples/avatar.ugoe` を写さずに使う。`vite.config.ts` の `sampleModel` が開発サーバーで
  `samples/avatar.ugoe` として配り、ビルドの出力に入れる (約 1.3 MB)。ブラウザでは `?model=samples/avatar.ugoe` でも開ける。
- パラメータの値域・既定値・グループは `WebPuppet.parameters_json()` から得る。

## 使い方

| 操作 | 内容 |
| --- | --- |
| 開く (Ctrl+O / ⌘O) | OS のダイアログで `.ugoe` か `.json` を選ぶ |
| サンプル | 同梱の見本のモデル (`assets/samples/avatar.ugoe`、アバタープロファイルのレベル 3、MIT) を開く。フェイストラッキングをすぐ試せる |
| ファイルをウィンドウにドロップ | そのファイルを開く |
| `ugoe-viewer path/to/model.ugoe` | 起動時に開く |
| ホイール | カーソルの位置を中心にズーム |
| ドラッグ (左・中ボタン) | パン |
| ダブルクリック・F・Fit | 全体を表示 |
| 背景・透明 | 背景色。透明にすると市松模様の上に描く |
| スライダー | パラメータ。ダブルクリックか ↺ で既定値、「すべて戻す」で全部を既定値に戻す。中継用のパラメータ (`internal`、物理のリグの間の受け渡し用) は既定で隠し、「中継用のパラメータも表示」で出す |

## フェイストラッキング (T0703)

右のパネルの「フェイストラッキング」で、カメラの映像から表情と頭の向き、体の向きと肩の上下 (T0645、「体 (肩) も」。
[packages/face-tracking](../../packages/face-tracking/README.md#体のトラッキング-t0645)) を取ってモデルを動かす。

- 顔の検出は [MediaPipe Face Landmarker](https://ai.google.dev/edge/mediapipe/solutions/vision/face_landmarker/web_js)
  (`@mediapipe/tasks-vision`) で行い、ARKit 互換のブレンドシェイプ (MediaPipe は `_neutral` と 51 個。`tongueOut` は無い) と
  顔の変換行列を得る。
- パラメータへの変換は Rust の対応付け (`ugoe_motion::tracking`、[アバタープロファイル](../../docs/spec/avatar-profile.md#フェイストラッキングとの対応))
  を `WebPuppet.set_tracking` で呼ぶ。TS では式を持たない。モデルの `avatar.tracking` の上書きも効く。
- アプリ側で行うこと (`@ugoe/face-tracking`。仕様の「実装の範囲」でホストの責務とされたもの):
  - 頭の向き: 変換行列 (列優先の 4x4) の回転を Y・X・Z の順に分解し、ヨー・ピッチ・ロール (度) にする (`frame.ts`)。
    反転していない映像では、写った人が自分の左を向くと正のヨー、上を向くと正のピッチ、画面で時計回りに傾くと正のロール。
  - キャリブレーション: 「キャリブレーション」で直近 15 フレームの平均を中立にし、頭の向きは差し引き、
    ブレンドシェイプは中立の値を 0、1 を 1 にする (`calibration.ts`)。
  - 平滑化: 値ごとに One Euro フィルタ (`filter.ts`)。「平滑化」の強さ (0〜1) で止まっているときの遮断周波数を 30 Hz〜0.3 Hz にする。0 なら素通し。
  - 顔を見失ったときは、最後の値を保つ。止めると、トラッキングが書き込んでいたパラメータを既定値に戻す。
  - 左右: 既定では、写った人自身の左右がキャラクター自身の左右になる (仕様の約束。カメラの映像は反転していないので、
    MediaPipe の値をそのまま使う)。「左右を入れ替える」を選ぶと、左右の名前とヨー・ロールの符号を入れ替え、モデルが鏡に映った
    自分のように動く。入力の映像がすでに反転しているとき (反転済みの仮想カメラなど) も、これで直す。
    「鏡像で表示」は映像の表示だけを反転し、値は変えない。
- **目パチ:** トラッキング中は、既定でモデルの自動の目パチを止める (「目パチを止める」、`WebPuppet.set_enabled("eye_blink", false)`)。
  目パチは基準値に重なるので、止めないとトラッキングの目の開閉と目パチが混ざる。止めると元に戻す。
- カメラの無い環境 (CI・ヘッドレス) では「ファイル…」で動画・画像、または記録した値の JSON を入れる。
  - 動画は繰り返し再生し、画像は同じ画像を検出し続ける。
  - 「記録」で検出した値 (キャリブレーション・平滑化の前) を記録し、「保存」で JSON にする。形式は `packages/face-tracking/src/recording.ts`
    (`{"format": "ugoe-face-tracking", "version": 2, "frames": [{"t", "head": {"yaw", "pitch", "roll"}, "blendShapes": {...}, "body": {...}}]}`。
    `body` は体のトラッキングの値で、版 2 で足した)。
    JSON を開くと MediaPipe を使わずに繰り返し再生する (キャリブレーションと平滑化は通す)。
  - ヘッドレスのブラウザからは `window.__UGOE_TRACKER__` (`FaceTracker`) を触れる。`output` (渡した値)・`landmarks`・`status` など。
  - Chrome の偽のカメラ (`--use-fake-device-for-media-stream --use-file-for-fake-video-capture=<file.y4m>`) でも、カメラの経路を確かめられる。

### MediaPipe の同梱

実行時にネットワークに頼らないよう、wasm とモデルをアプリに同梱する (`public/mediapipe/`)。

- wasm は npm パッケージの `wasm/` から写す (SIMD 版と SIMD の無い版。`FilesetResolver` が選ぶ)。
- モデル `face_landmarker.task`・`pose_landmarker_lite.task` は **リポジトリにコミットしない**。`ugoe-mediapipe-assets` (`packages/face-tracking/scripts/mediapipe-assets.mjs`) が版を固定した URL
  (`.../face_landmarker/float16/1/face_landmarker.task`・`.../pose_landmarker_lite/float16/1/pose_landmarker_lite.task`) から取得し、SHA-256 を検証する (違えば失敗する)。
  検証済みのファイルがあれば取得しない。モデルを更新するときは、スクリプトの URL とハッシュを一緒に書き換える。
- `pnpm build` (= `tauri build` の前) は取得できなければ失敗する。`pnpm dev` は取得できなくても警告だけで起動する
  (トラッキングを始めると「モデルが同梱されていません」と表示する)。手で用意するときは `pnpm mediapipe`。
- ライセンスは両方とも Apache-2.0。表記は `public/third-party-notices.txt` にあり、アプリのステータスバーの「ライセンス」で表示する。

### カメラの許可 (OS ごと)

| OS | webview | 設定 | 確かめたか |
| --- | --- | --- | --- |
| Linux | WebKitGTK | バックエンドの `on_permission_request` でカメラの要求だけを許す (`src-tauri/src/lib.rs`)。ハンドラが無いと WebKitGTK は要求を拒む | WSLg (WebKitGTK 2.52、カメラ無し): 画像での検出とモデルへの反映、`getUserMedia` が呼べること (カメラが無いので `Invalid constraint` で失敗) まで。実際のカメラでは未確認 |
| Windows | WebView2 | 同じハンドラで許す (WebView2 は既定では利用者に尋ねる) | 未確認 |
| macOS | WKWebView | `src-tauri/Info.plist` の `NSCameraUsageDescription` (OS が初回に尋ねる) と、Hardened Runtime 向けの `src-tauri/Entitlements.plist` (`com.apple.security.device.camera`) | 未確認 |

MediaPipe には WebGL の canvas を明示して渡す (`tracker.svelte.ts`)。渡さないと OffscreenCanvas を使い、
WebKitGTK では検出のときに `GLctx.activeTexture` のエラーで失敗した。

確かめていない OS では、利用者が次を確認する: 「開始」でカメラの許可を求められる (または自動で許される) こと、カメラの一覧に名前が出ること、
映像とランドマークが表示され、目の開閉・口の開き・頭の向きがモデルに反映されること。

## 外からパラメータを動かす (ドライバ)

フレームごとにパラメータを設定するものは `ParameterDriver` (`src/lib/drivers.ts`) として登録する。
フェイストラッキング (T0703) や、自動の目パチ・呼吸の確認はこの形で足す。

```ts
const remove = player.drivers.add({
  name: "sway",
  frame(dt) {
    t += dt;
    return [["AngleX", 30 * Math.sin(t)]];
  },
});
```

- 毎フレーム、登録順にすべてのドライバの `frame(dt)` を呼び、返った値を設定してから `update(dt)`・描画する。
  同じパラメータを複数のドライバが返したら、後に登録したものが勝つ。
- フェイストラッキングはドライバではなく `Player.setTracking` で渡す (Rust の `Puppet::set_tracking`)。
  `update(dt)` の中で書き込むので、トラッキングが書き込むパラメータではドライバの値より優先される。
- ドライバが値を返しているパラメータのスライダーは「(自動)」と表示され、手では動かせない。
- 開発中やヘッドレスのブラウザでは、表示中の `Player` を `window.__UGOE_VIEWER__` から触れる。

右のパネル (`App.svelte` の `.sidebar`) は縦にパネルを積む作りで、トラッキングのパネルはパラメータの上にある。

## ヘッドレスのブラウザで確かめる

Tauri の外 (ふつうのブラウザ) で開くと、ファイルの選択は `<input type="file">` になる。さらに:

- `?model=<URL>` を付けると、起動時にその URL のモデルを開く (例: `http://localhost:1430/?model=/c.ugoe`)。
- `window.__UGOE_VIEWER_HOST__` に `ViewerHost` (`src/lib/host.ts`) を置くと、「開く」などでそれを使う。
- ドロップは `DragEvent` と `DataTransfer` に `File` を入れて `window` に送れば確かめられる。
- WebGPU の無いヘッドレスの Chrome でも、SwiftShader の WebGL2 で描ける
  (`--use-gl=angle --use-angle=swiftshader --enable-unsafe-swiftshader`)。

## コマンド

```sh
just viewer                      # 開発モードで起動 (vite の 1430 番ポート + tauri dev)
just viewer-build                # リリースビルド (インストーラを作る)。例: just viewer-build --bundles deb
pnpm --filter @ugoe/viewer wasm  # ugoe-wasm をビルドして src/lib/wasm/generated/ に JS バインディングを出す
pnpm --filter @ugoe/viewer mediapipe  # MediaPipe の wasm とモデルを public/mediapipe/ に用意する (dev・build でも実行される)
just web-check                   # Biome + svelte-check + Vitest (エディタとビューアの両方)
```

`just viewer` と `just viewer-build` は、先に `pnpm wasm` を実行する (`tauri.conf.json` の `beforeDevCommand` /
`beforeBuildCommand`)。必要な道具 (Rust・Node.js・pnpm・just・wasm-bindgen-cli・各 OS の前提パッケージ) はエディタと同じなので、
[apps/editor/README.md#各-os-でのビルド](../editor/README.md#各-os-でのビルド) を参照。
GitHub Actions の Editor build のワークフローは、ビューアのインストーラ (`ugoe-viewer-<OS>`) も作る。

アイコンは `src-tauri/icons/icon.png` から `pnpm tauri icon src-tauri/icons/icon.png -o src-tauri/icons` で作り直せる
(Android・iOS 用に出力されるディレクトリは使わないので消す)。
