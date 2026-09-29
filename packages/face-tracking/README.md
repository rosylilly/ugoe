# packages/face-tracking

ビューア (`apps/viewer`) とエディタのプレビュー (`apps/editor`) で共有するフェイストラッキングの部品
(npm パッケージ `@ugoe/face-tracking`、pnpm のワークスペース)。T0703 でビューアに作ったものを T0636 で切り出した。

**ライセンスは GPL-3.0-only** (使うアプリと同じ)。同梱する MediaPipe は Apache-2.0 ([third-party-notices.txt](third-party-notices.txt))。

```
src/frame.ts            MediaPipe の結果 → TrackingFrame (頭の向きとブレンドシェイプ)、左右の入れ替え
src/calibration.ts      キャリブレーション (中立の顔を差し引く)
src/filter.ts           One Euro フィルタによる平滑化
src/recording.ts        記録した値の列 (JSON) の形式・再生
src/tracker.svelte.ts   FaceTracker: 入力 (カメラ・動画・画像・記録) → MediaPipe → 上の処理 → TrackingTarget
src/TrackingPanel.svelte  パネル (入力の選択・映像とランドマーク・調整・記録・値の表示)
src/strings.ts          画面の文言 (日本語 `ja` / 英語 `en`)
scripts/mediapipe-assets.mjs  `ugoe-mediapipe-assets <出力先> [--optional]`: MediaPipe の wasm を写し、モデルを取得して
                        SHA-256 を検証し、ライセンスの表記を写す
```

## 使い方

- アプリは `TrackingTarget` (`setTracking(frame | null)` と `setAutoEyeBlink(enabled)`) を実装して `FaceTracker` に渡す。
  値は次のフレームで `WebPuppet.set_tracking` に渡す (パラメータへの変換は Rust の対応付け)。`null` を受けたら
  `clear_tracking` し、トラッキングが書き込んでいたパラメータを既定値に戻す。
- `new FaceTracker(target, { strings, saveJson })`。`saveJson` を省くとブラウザのダウンロードで保存する。
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
