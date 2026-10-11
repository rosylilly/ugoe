# packages/face-tracking

ビューア (`apps/viewer`) とエディタのプレビュー (`apps/editor`) で共有するフェイストラッキングの部品
(npm パッケージ `@ugoe/face-tracking`、pnpm のワークスペース)。T0703 でビューアに作ったものを T0636 で切り出した。

**ライセンスは GPL-3.0-only** (使うアプリと同じ)。同梱する MediaPipe は Apache-2.0 ([third-party-notices.txt](third-party-notices.txt))。

```
src/frame.ts            MediaPipe の結果 → TrackingFrame (頭の向き・ブレンドシェイプ・体)、左右の入れ替え、
                        WebPuppet.set_tracking への受け渡し (setPuppetTracking)
src/pose.ts             Pose Landmarker の点 → 体の値 (体の向きと肩の上下、T0645)
src/calibration.ts      キャリブレーション (中立の顔を差し引く)
src/tuning.ts           利用者の調整 (首振りの感度と遊び・瞼と口の開きの境目。T0877)
src/filter.ts           One Euro フィルタによる平滑化
src/recording.ts        記録した値の列 (JSON) の形式・再生
src/tracker.svelte.ts   FaceTracker: 入力 (カメラ・動画・画像・記録) → MediaPipe → 上の処理 → TrackingTarget
src/status.ts           パネルの状態の文 (顔が見つからないなど) と、その揺れの抑え (T0958)
src/TrackingPanel.svelte  パネル (入力の選択・映像とランドマーク・調整・記録・値の表示)
src/TuningSection.svelte  パネルの中の「調整 (感度と境目)」(スライダーと、調整の前と後のメーター)
src/usage-logging.ts    MediaPipe の利用状況の送信を断る (下の「利用状況の送信」)
src/mediapipe-bundle.test.ts  入れている MediaPipe の配布物が、上の止め方の頼る形で送るままかを確かめる
src/i18n.ts             画面の文言 (@ugoe/i18n の名前空間 `face-tracking`。カタログは locales/<言語>.json、原文は英語)
scripts/mediapipe-assets.mjs  `ugoe-mediapipe-assets <出力先> [--optional]`: MediaPipe の wasm を写し、モデルを取得して
                        SHA-256 を検証し、ライセンスの表記を写す
```

## 使い方

- アプリは `TrackingTarget` (`setTracking(frame | null)` と `setAutoEyeBlink(enabled)`) を実装して `FaceTracker` に渡す。
  値は次のフレームで `setPuppetTracking(puppet, frame)` (= `WebPuppet.set_tracking`) に渡す (パラメータへの変換は Rust の対応付け)。`null` を受けたら
  `clear_tracking` し、トラッキングが書き込んでいたパラメータを既定値に戻す。
- `new FaceTracker(target, { saveJson, schedule })`。`saveJson` を省くとブラウザのダウンロードで保存する。
  `schedule` は入力の処理を繰り返し呼ぶ仕組み (止める関数を返す)。省くと `requestAnimationFrame` (`animationFrames`) で、ページが隠れると止まる。
  スタジオは最小化しても止まらないように、バックエンドの刻みで回す (T0803、docs/studio.md#入力)。
- 文言の言語は、アプリが `@ugoe/i18n` の `setLocale()` で決める (どのアプリも設定の言語。既定は OS に合わせる)。
  パネルの文言は言語の切り替えに追従する。エラーの文は起きた時の言語のまま。
- パネルは `import { TrackingPanel } from "@ugoe/face-tracking"` (`index.ts` から。サブパスで import すると
  svelte-check が同じ型を別物として扱うため)。パネルの部品と見た目は `@ugoe/ui` (Section・Select・Toggle・Slider・Button・Tooltip。T0907)。
  アプリは `@ugoe/ui` の `tokens.css` を読んでおく。
- パネルは高さの上限と中の送りを持たず、中身の高さで伸びる (T0951)。置く側のアプリが、並べるほかの部品と合わせて
  高さと送りを決める (ビューアは右の列の上の段がまとめて送り、スタジオの右の列・エディタのプレビューの列はそれぞれ列ごと送る)。
- アプリの `pnpm dev` / `pnpm build` の前に `ugoe-mediapipe-assets public/mediapipe` を実行する (`dev` は `--optional`)。
  `public/mediapipe/` はコミットしない。実行時は `document.baseURI` からの `mediapipe/` を読む。
- `FaceTracker` は MediaPipe を読む前に、ページの `fetch` を包んで MediaPipe の利用状況の送信を断る (下の「利用状況の送信」)。

## コマンド

```sh
pnpm --filter @ugoe/face-tracking test    # Vitest (純粋な関数)
pnpm --filter @ugoe/face-tracking check   # svelte-check
```

どちらも `just web-check` (`just ci`) に入っている。

## 体のトラッキング (T0645)

顔と同じフレームで MediaPipe Pose Landmarker (lite) を動かし、体の向き (ヨー・ピッチ・ロール) と肩の上下を
`TrackingFrame.body` に入れる。式・キャリブレーションの扱い・確かめたことは
[docs/spec/avatar-profile.md#体のトラッキング-mediapipe-pose-landmarker](../../docs/spec/avatar-profile.md#体のトラッキング-mediapipe-pose-landmarker)。

- パネルの「体 (肩) も」で切れる (既定は入れる)。切ると `body` を省き、Rust の対応付けは体を頭 × 0.3 にする。
- 体の値は、無い値を `null` にする (肩が見えない・腰が見えない・キャリブレーションしていない)。ピッチと肩の上下は
  キャリブレーションした後だけ渡す (`calibration.ts`)。
- モデル `pose_landmarker_lite.task` (5.5 MiB) も `ugoe-mediapipe-assets` が取得・検証する。無ければ顔だけでトラッキングを続ける。
- 記録の形式は版 2 (`body` が増えた。版 1 も読める)。

## 利用者の調整 (T0877)

キャリブレーションの後・平滑化の前に、利用者の顔・カメラ・部屋に合わせる調整を当てる (`applyTuning`。
[docs/spec/avatar-profile.md#実装の範囲](../../docs/spec/avatar-profile.md#実装の範囲) でアプリの責務とされたもの)。
モデルの `avatar.tracking` の上書き (モデルを作る人が決める) とは別。

```
検出 → (左右を入れ替えるなら unmirrorFrame) → 記録 → applyCalibration → applyTuning → FrameSmoother → TrackingTarget
```

| 項目 | 値 | 当て方 |
| --- | --- | --- |
| `yaw`・`pitch`・`roll` | `gain` (感度。0〜4、既定 1)・`deadZone` (遊び。度、0〜20、既定 0) | `sign(v) × max(0, abs(v) − deadZone) × gain`。遊びの中は 0、外は 0 から続けてつなぐ |
| `eyeBlink` (`eyeBlinkLeft`・`eyeBlinkRight`)・`jawOpen` | `low`・`high` (0〜1、既定 0 と 1。間は 0.05 以上) | `clamp((v − low) / (high − low), 0, 1)` |

- **既定値なら何もしない:** `DEFAULT_TUNING` のまま (倍率 1・遊び 0・境目 0 と 1) なら、`applyTuning` は受け取ったフレームをそのまま返す
  (出力はこの機能の前とビット単位で同じ)。項目ごとにも、既定値の項目は値に触らない。
- **壊れた値:** `sanitizeTuning` が、無い・知らない項目を捨てて既定値にし、NaN・無限大は既定値、範囲の外は端に寄せ、間が 0.05 より狭い
  (下が上以上を含む) 境目の組は既定に戻す。`FaceTracker.setTuning` と `applyTuning` の両方で通す。画面のスライダーは片方の境目を
  動かすと、間を保つようにもう片方を押す (`moveRangeBound`。動かした方は利用者の決めた値のまま)。
- **左右:** 瞼は左右を 1 つの組で一緒に動かす (下の「決めたこと」)。左右の入れ替え (`swapSides`) は検出の直後に済んでいるので、調整は利用者から見た
  左右 (出力の左右) に当たる。
- **変えないもの:** 体 (`body`) と、組に無いブレンドシェイプ (`mouthClose` を含む。`MouthOpenY` = `max(0, jawOpen − mouthClose)` の
  `jawOpen` だけを伸ばす)。体を使っていないときの体の回転 (Rust の対応付けの頭 × 0.3) は、調整した後の頭から作られる。
- **記録:** 記録するのは調整の前 (キャリブレーションの前) の値。再生しながら調整し直せる (キャリブレーションと同じ)。
- **広げるとき:** `TUNING_RANGES` にまとまりとブレンドシェイプの名前を足し、`TrackingTuning`・`DEFAULT_TUNING`・`sanitizeTuning`・
  スタジオの `input.rs` の `TrackingTuning`・パネル (`TuningSection.svelte` の `RANGES`) と文言に項目を足す。
- **保存:** パッケージは持つだけで保存しない。スタジオはプロファイルの `input.tuning` に保存する ([docs/studio.md](../../docs/studio.md#入力とホットキーの項目-t0803))。
  ビューアは `localStorage` の `ugoe.viewer.tracking.tuning` に覚える ([apps/viewer/README.md](../../apps/viewer/README.md#フェイストラッキング-t0703)。T0738)。
  エディタは保存しない (開き直すと既定値)。
- **決めたこと:**
  - 瞼の左右を一緒にした: 細目の度合いは左右でほぼ同じで、検出の左右差は首の向きで変わる (向きで揺れる差を固定の値で直すと、正面で
    片目だけ閉じきる) ため。スライダーの数も半分になる。左右を別にするなら `eyeBlinkLeft`・`eyeBlinkRight` の組を分けて足す。
  - 遊びは「引いてつなぐ」形 (遊びの境で値が飛ばない)。感度は遊びを引いた後に掛ける (遊びの角度が感度で変わらない)。
  - 境目は 2 つの値 (下と上) にした: 「開いた」と「閉じた」の両方を決めたい (細目の人は閉じの値が 0 まで下がらず、1 まで上がらない) ため。

## 利用状況の送信 (T0943)

カメラの映像・画像・ランドマークはこの計算機の中で処理し、どこにも送らない。MediaPipe (`@mediapipe/tasks-vision` 1.0.1) は
利用状況 (メトリクス) を Google へ送る仕組みを持つが、このパッケージが断るので送られない。

MediaPipe がすること (npm パッケージの `vision_bundle.mjs` を読んで確かめた。出典は
[MediaPipe Tasks のプライバシーの説明](https://goo.gle/mediapipe-privacy)・npm パッケージの README の「Privacy Notice」):

- いつ: `FaceLandmarker`・`PoseLandmarker` などを `createFromOptions` で作ると、必ず記録を始める (作った直後に 1 件溜まる)。
  溜めた記録を 60 秒ごとと、Landmarker を `close()` したときに送る。送るのに 1 回失敗すると、溜めた記録を捨て、以後は送らない。
- 何を: タスクの種類 (`FaceLandmarker` など)・実行モード (`VIDEO` など)・OS の種類 (user agent から)・MediaPipe の版・
  検出の回数と処理時間の統計・落としたフレームの数。映像・画像・ランドマーク・ブレンドシェイプの値は含まない。
- どこへ: `https://odml.pa.googleapis.com/v1/log` に `fetch` で POST する (protobuf。API キーは wasm から取る)。
- 止める設定: 無い。オプション (`baseOptions` など) にも `FilesetResolver` にも無く、送り先と送る処理は JS の側に決め打ちなので、
  wasm の版 (SIMD・SIMD 無し・`module`) や `ModuleFactory` の差し替えでも止まらない。

止め方: `src/usage-logging.ts` の `blockMediaPipeUsageLogging()` が、グローバルの `fetch` を包み、`https://odml.pa.googleapis.com`
への要求だけをネットワークの失敗 (`TypeError`) として断る (ほかの要求はそのまま通す)。MediaPipe は送るたびにグローバルの `fetch` を
引くので包んだものを呼び、失敗として扱って以後は送らない。要求はネットワークに出ないので、CSP の違反にもならない。
`FaceTracker` が Landmarker を作る前に呼ぶ (何度呼んでも包むのは 1 回)。

二重の守りとして、3 つのアプリの CSP (`connect-src`) は `odml.pa.googleapis.com` を許していない (仮に包みが外れても CSP が止める)。
e2e の画像からのトラッキングの手順は、ページの `fetch` がこの送り先を断ることと、CSP の違反が 0 件であることを確かめる
([tests/e2e/README.md](../../tests/e2e/README.md#画像からのトラッキング))。

MediaPipe を自分で使うページ (`@ugoe/runtime` の `<ugoe-avatar>` に MediaPipe の結果を渡すホストなど) は、このパッケージを通らないので
送信が止まらない。止めるには、そのページで同じように `fetch` を包むか、CSP の `connect-src` で送り先を許さない
([bindings/web/README.md](../../bindings/web/README.md#mediapipe-の利用状況の送信))。

### MediaPipe の版を上げるとき

`src/mediapipe-bundle.test.ts` (Vitest。`just ci` の `web-check` で走る) が、入れている `@mediapipe/tasks-vision` の配布物
(`vision_bundle.mjs`・`.cjs`・`.js` と `wasm/` の中身) を読み、止め方が頼っている形のままかを確かめる。

- 配布物の中の URL が送り先 `https://odml.pa.googleapis.com/v1/log` の 1 つだけで、`usage-logging.ts` が断るオリジンであること
- 送る処理がグローバルの `fetch` を呼ぶたびに引くこと (`fetch(<要求>.url, …)` があり、`fetch` を変数に取っておいたり
  `bind` したりしていない)。`sendBeacon`・`XMLHttpRequest`・`WebSocket`・`EventSource` を使わないこと
- `wasm/` の JS と wasm が送り先を持たないこと (送り先は JS の側だけ)

版を上げる手順:

1. `package.json` の版を上げて `pnpm install` し、`pnpm --filter @ugoe/face-tracking test` を流す。
2. 落ちたら、その配布物を `node_modules` の中で読み (リポジトリには写さない)、送り先・送る時期・送り方を確かめる。
   止め方が効かなくなったなら `src/usage-logging.ts` を直し、この節の「MediaPipe がすること」とテストの期待を合わせる。
3. 通っても、[プライバシーの説明](https://goo.gle/mediapipe-privacy) と npm パッケージの README の「Privacy Notice」が
   変わっていないかを読み、この節の版 (1.0.1) を書き換える。e2e (`just e2e`) で CSP の違反が無いことも確かめる。
4. `bindings/web` の見本 (`examples/basic`) の MediaPipe の版 (CDN の URL) と、bindings/web/README.md の
   「MediaPipe の利用状況の送信」の版も合わせる (見本は自分で `fetch` を包む。このパッケージには依存しない)。
