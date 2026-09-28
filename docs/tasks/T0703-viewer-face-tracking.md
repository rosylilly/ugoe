---
id: T0703
title: ビューア: MediaPipe の顔のトラッキングで表情を付ける
status: done
milestone: M3
crates: [apps/viewer, ugoe-wasm, docs]
depends_on: [T0701, T0306]
assignee: claude-worker-T0703
---

# T0703: ビューア: MediaPipe の顔のトラッキングで表情を付ける

## 目的

ビューア (T0701) で、カメラの映像から MediaPipe Tasks の Face Landmarker で ARKit 互換の 52 ブレンドシェイプと頭の向きを得て、
アバタープロファイルの対応付け (T0306) でパラメータに変換し、ARKit 互換の顔のモデル (T0702) に表情が付くことを確かめられるようにする。

## 受け入れ条件

- [x] `@mediapipe/tasks-vision` の Face Landmarker (`outputFaceBlendshapes`、`outputFacialTransformationMatrixes`) を使う。wasm とモデル (`face_landmarker.task`) をアプリに同梱し、実行時にネットワークに頼らない (ライセンス (Apache-2.0) の表記を同梱する。モデルのファイルはリポジトリにコミットせず、ビルド時に取得して検証する方法を決めて書く)
- [x] 変換は T0306 の Rust の実装を wasm から呼ぶ (TS で式を書き直さない)。頭の向きは変換行列からヨー・ピッチ・ロールを求め、符号をプロファイルの約束に合わせる
- [x] 画面: カメラの選択、映像 (鏡像の切り替え)、ランドマークの重ね描き、52 の値と変換後のパラメータの値の表示、トラッキングの開始・停止、平滑化 (強さを調整)、キャリブレーション (今の顔を中立にする)
- [x] カメラがない環境 (CI・ヘッドレス) でも確かめられる入力: 動画・画像ファイル、または記録した値の列 (JSON) の再生
- [x] 実際の人の顔の動画か画像で、目の開閉・口の開き・頭の向きが見本のモデル (T0702) に反映されることを確かめ、メモに書く。符号の約束 (`EyeBallX`・`MouthX`・`JawX`) を確かめられたら T0306 の未決の解消に使う
- [x] Tauri でカメラを使うための設定 (macOS の `NSCameraUsageDescription`、Linux の WebKitGTK・Windows の WebView2 での許可) を決めて書き、確かめられなかった OS は利用者の確認項目にする
- [x] `just ci`・`just licenses`・ビューアのビルドが通る

## 進め方

見本のモデル (T0702) は確認にだけ使うので、T0702 を待たずに着手する (オーケストレーターの判断)。T0702 が main に入るまでは `ugoe_model::fixtures::avatar_profile` などで確かめ、入ったら見本のモデルで確かめ直す。

## 参考

- https://ai.google.dev/edge/mediapipe/solutions/vision/face_landmarker/web_js
- `docs/spec/avatar-profile.md#フェイストラッキングとの対応`

## メモ

### やったこと

- **ugoe-wasm** (`src/tracking.rs`・`src/web/tracking.rs`。T0309 と衝突しないよう別のファイル): `WebPuppet.set_tracking(names, scores, yaw, pitch, roll)`
  (MediaPipe のカテゴリ名をそのまま渡し、`TrackingInput::set_blend_shapes` → `Puppet::set_tracking`)、`clear_tracking`、`is_tracking`、
  `tracking_parameter_ids` (`TrackingMap::targets` をモデルの定義順に)。式は TS に書いていない。`WebPuppet` はもともと `Puppet::update` を通していた
  (T0309 で整理済み)。目パチの停止は最初 `set_auto_eye_blink` (effects を差し替え) で作ったが、T0309 の `set_enabled("eye_blink", …)` が入ったので
  それに置き換えて消した。
- **ビューア** (`apps/viewer/src/lib/tracking/`・`TrackingPanel.svelte`): 入力 (カメラ・動画・画像・記録の JSON) → MediaPipe → 変換行列からヨー・ピッチ・ロール →
  (選べば) 左右の入れ替え → 記録 → キャリブレーション → One Euro の平滑化 → `Player.setTracking` → `WebPuppet.set_tracking`。
  画面はカメラの選択・開始/停止・「ファイル…」・映像とランドマーク (鏡像の表示の切り替え)・左右の入れ替え・目パチの停止・平滑化の強さ・
  キャリブレーション/解除・記録/保存・ブレンドシェイプ (51 個 + 頭の向き) と変換後のパラメータの値。ステータスバーに「ライセンス」。
  - 目パチ: トラッキング中は既定で止める (チェックで切り替え)。止めないとトラッキングの目の開閉にモデルの目パチが重なる。
  - 顔を見失ったら最後の値を保つ。止めるとトラッキングが書いていたパラメータを既定値に戻す (`None` にしても最後の値が基準値に残るため)。
  - 左右: 既定は仕様どおり (写った人の左右 = キャラクターの左右)。「左右を入れ替える」で鏡のように動かす (反転済みの入力を直すのにも使う)。
- **MediaPipe の同梱:** `@mediapipe/tasks-vision` 1.0.1 (Apache-2.0)。`scripts/mediapipe-assets.mjs` が wasm (SIMD 版と SIMD の無い版) を写し、モデルを
  `.../face_landmarker/float16/1/face_landmarker.task` から取って SHA-256 (`64184e22…`) を検証する。`public/mediapipe/` は gitignore。`pnpm build` は取れなければ失敗、
  `pnpm dev` は警告だけ。Apache-2.0 の全文と出典を `public/third-party-notices.txt` に書き、アプリで表示する。.deb は約 4 MiB → 約 13 MiB になった (wasm 約 22 MiB + モデル 3.6 MiB、圧縮後)。
- **Tauri:** `on_permission_request` でカメラだけ許す、記録の保存に `write_json_file` (`.json` だけ) と `dialog:allow-save`、macOS の `Info.plist` (`NSCameraUsageDescription`) と
  `Entitlements.plist` (`com.apple.security.device.camera`、`bundle.macOS.entitlements`)。
- **WebKitGTK の不具合を 1 つ直した:** MediaPipe に canvas を渡さないと OffscreenCanvas の WebGL を使い、Tauri (WebKitGTK) では検出のときに
  `GLctx.activeTexture` のエラーになった。ふつうの canvas を明示して渡すと動いた。
- 「サンプル」ボタン: 同梱の見本のモデル (`assets/samples/avatar.ugoe`、MIT) を開く。リポジトリのファイルを写さず、`vite.config.ts` のプラグインが配る・出力に入れる。
- 範囲外の小さな変更: `apps/editor/src/lib/viewport/ugoe-wasm-stub.d.ts` に `set_tracking` など 4 行 (stub を web.rs と揃える約束)。`.gitignore` に 1 行。
- 仕様: `docs/spec/avatar-profile.md` の未決を更新 (下)。MediaPipe に `tongueOut` が無いことも書いた。

### 実際の顔で確かめたこと

素材はリポジトリの外 (scratchpad) に置き、コミットしていない: Wikimedia Commons の Clément Bucco-Lechat の表情の動画 (CC BY-SA 3.0: Blinking・Badly surprised
(片目のウインク)・Va et vient (視線を左右に)・Surprised・Rolled tongue)、あくびの写真 (CC BY 2.0 ×2)、MediaPipe の試験用の portrait.jpg (米政府の肖像写真) と、それを左右反転・20° 回転したもの。
ヘッドレスの Chrome (SwiftShader) でビルドした `dist/` を開き、「ファイル…」に入れて、見本のモデル (T0702 の `assets/samples/avatar.ugoe`、レベル 3) と `avatar_profile` のフィクスチャで確かめた。
最初は T0702 の作業中のファイルで確かめ、T0702 が main に入った後に、main の `assets/samples/avatar.ugoe` (作業中のファイルとバイト単位で同じ) を「サンプル」ボタンで開いて確かめ直した (値は下のとおり)。

- **目の開閉:** Blinking で `EyeLOpen` 0.22〜0.86 (キャリブレーション後は 0.42〜1.0)。モデルの目が閉じる。ウインク (画像の左の目) では `eyeBlinkRight` 0.55 / `eyeBlinkLeft` 0.26 で `EyeROpen` が下がる。
- **口の開き:** あくびの写真で `MouthOpenY` 0.85 / 0.54。モデルの口が開く。
- **頭の向き:** 顔が画像の右を向いた写真 (鼻先が顔の中心より右) でヨー +28° / +16°、左右反転で符号が反転。20° 時計回りに回した画像でロール +19.5°。ピッチはランドマークの奥行き (額とあご) から求めた向きと動画で相関 0.89〜0.99。
- **視線:** Va et vient で、虹彩が画像の左に寄ると `EyeBallX` −0.9、右に寄ると +0.9。モデルの目が動く。
- **符号の結論 (T0306 の未決):** MediaPipe の Left/Right は写った人自身の左右。`EyeBallX` は `AngleX` と同じ「自分の左 = 正」で、反転しない映像ではキャラクターは写った人と同じ向きを見る。仕様の約束のままでよい (反転は不要)。
  `MouthX` / `JawX` は口やあごを横に動かす素材が無く **確かめられなかった** (仕様に未決として残した)。
- カメラの経路: Chrome の偽のカメラ (`--use-file-for-fake-video-capture`、Va et vient の y4m) で「開始」→ カメラの一覧・映像・検出・`EyeBallX` ±0.9。
- キャリブレーション・記録 (36 フレーム) → 保存 (ダウンロード)・停止 (既定値に戻り、目パチが有効に戻る)・記録の再生・ライセンスの表示も確かめた。
- **Tauri (WSLg、WebKitGTK 2.52、`target/release/ugoe-viewer assets/samples/avatar.ugoe`):** 起動時の引数で見本のモデルを開き、OS のダイアログで画像を開いて検出 (6 fps) し、Chrome と同じ値 (AngleX 16.2、AngleZ −18.8) がモデルに反映された。
  「開始」は `getUserMedia` まで届き、カメラが無いので `Invalid constraint` で失敗した (WSL にカメラが無い)。ライセンスの表示も確かめた。
  スクリーンショットで、あくびの写真に合わせてモデルの目が閉じ、口が開き、頭が傾くことを見た。「サンプル」ボタンでも開けた。
- `just ci`・`just licenses`・`just wasm-check`・`just viewer-build --bundles deb` が通る。

### 利用者に確かめてほしいこと

- 実際のカメラで: Windows (WebView2)・macOS (WKWebView、初回に OS の許可を求められること)・Linux (WebKitGTK) で「開始」→ 許可 → カメラの一覧に名前が出る → 映像とランドマーク →
  目の開閉・口の開き・頭の向き・視線がモデルに反映される。macOS は署名 (Hardened Runtime) したビルドでもカメラが使えるか。
- `MouthX` / `JawX` の符号 (口とあごを自分の左に動かして正になるか)。
- 「左右を入れ替える」を切ったとき・入れたときの動きのどちらが自然か (既定は仕様どおり入れ替えない)。
- 平滑化の強さの既定 (0.5) と One Euro の係数 (`filter.ts`) が実際のカメラで適切か。

### やらなかったこと・後続の候補

- トラッキングが途切れたときに中立へ戻すフェード (今は最後の値を保つ)。体のトラッキング (体の回転は頭 × 0.3)。
- 表示のモデルを鏡像にする切り替え (仕様の「表示側で反転する」の実装)。今は入力の左右の入れ替えで代用できる。
- ヘッドレスの Chrome では 9 fps (SwiftShader・CPU)。実機の GPU で速さを確かめる。
- `lefthook.yml` の pre-push の web-check はまだエディタだけ (T0701 から)。
- 利用者の確認 (2026-09-29): 実際のカメラでトラッキングが動き、モデルに表情が付くことを確かめた。見本のモデル (T0702) の
  動きが不自然で細かい確認 (`MouthX`・`JawX` の符号、平滑化の既定値など) はしきれていない。利用者が、よりよい見本の元になる PSD を用意している。
