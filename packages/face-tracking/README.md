# packages/face-tracking

ビューア (`apps/viewer`) とエディタのプレビュー (`apps/editor`) で共有するフェイストラッキングの部品
(npm パッケージ `@ugoe/face-tracking`、pnpm のワークスペース)。T0703 でビューアに作ったものを T0636 で切り出した。

**ライセンスは GPL-3.0-only** (使うアプリと同じ)。同梱する MediaPipe は Apache-2.0 ([third-party-notices.txt](third-party-notices.txt))。

```
src/frame.ts            MediaPipe の結果 → TrackingFrame (頭の向き・ブレンドシェイプ・体)、左右の入れ替え、
                        WebPuppet.set_tracking への受け渡し (setPuppetTracking)
src/pose.ts             Pose Landmarker の点 → 体の値 (体の向きと肩の上下、T0645)
src/calibration.ts      キャリブレーション (中立の顔を差し引く)
src/filter.ts           One Euro フィルタによる平滑化
src/recording.ts        記録した値の列 (JSON) の形式・再生
src/tracker.svelte.ts   FaceTracker: 入力 (カメラ・動画・画像・記録) → MediaPipe → 上の処理 → TrackingTarget
src/TrackingPanel.svelte  パネル (入力の選択・映像とランドマーク・調整・記録・値の表示)
src/i18n.ts             画面の文言 (@ugoe/i18n の名前空間 `face-tracking`。カタログは locales/<言語>.json、原文は英語)
scripts/mediapipe-assets.mjs  `ugoe-mediapipe-assets <出力先> [--optional]`: MediaPipe の wasm を写し、モデルを取得して
                        SHA-256 を検証し、ライセンスの表記を写す
```

## 使い方

- アプリは `TrackingTarget` (`setTracking(frame | null)` と `setAutoEyeBlink(enabled)`) を実装して `FaceTracker` に渡す。
  値は次のフレームで `setPuppetTracking(puppet, frame)` (= `WebPuppet.set_tracking`) に渡す (パラメータへの変換は Rust の対応付け)。`null` を受けたら
  `clear_tracking` し、トラッキングが書き込んでいたパラメータを既定値に戻す。
- `new FaceTracker(target, { saveJson })`。`saveJson` を省くとブラウザのダウンロードで保存する。
- 文言の言語は、アプリが `@ugoe/i18n` の `setLocale()` で決める (エディタは英語、ビューアは日本語)。
  パネルの文言は言語の切り替えに追従する。エラーの文は起きた時の言語のまま。
- パネルは `import { TrackingPanel } from "@ugoe/face-tracking"` (`index.ts` から。サブパスで import すると
  svelte-check が同じ型を別物として扱うため)。パネルの見た目の変数 (`--muted` など) はアプリの CSS のもの。
- アプリの `pnpm dev` / `pnpm build` の前に `ugoe-mediapipe-assets public/mediapipe` を実行する (`dev` は `--optional`)。
  `public/mediapipe/` はコミットしない。実行時は `document.baseURI` からの `mediapipe/` を読む。

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
