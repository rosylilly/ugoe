---
id: T0703
title: ビューア: MediaPipe の顔のトラッキングで表情を付ける
status: in-progress
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

- [ ] `@mediapipe/tasks-vision` の Face Landmarker (`outputFaceBlendshapes`、`outputFacialTransformationMatrixes`) を使う。wasm とモデル (`face_landmarker.task`) をアプリに同梱し、実行時にネットワークに頼らない (ライセンス (Apache-2.0) の表記を同梱する。モデルのファイルはリポジトリにコミットせず、ビルド時に取得して検証する方法を決めて書く)
- [ ] 変換は T0306 の Rust の実装を wasm から呼ぶ (TS で式を書き直さない)。頭の向きは変換行列からヨー・ピッチ・ロールを求め、符号をプロファイルの約束に合わせる
- [ ] 画面: カメラの選択、映像 (鏡像の切り替え)、ランドマークの重ね描き、52 の値と変換後のパラメータの値の表示、トラッキングの開始・停止、平滑化 (強さを調整)、キャリブレーション (今の顔を中立にする)
- [ ] カメラがない環境 (CI・ヘッドレス) でも確かめられる入力: 動画・画像ファイル、または記録した値の列 (JSON) の再生
- [ ] 実際の人の顔の動画か画像で、目の開閉・口の開き・頭の向きが見本のモデル (T0702) に反映されることを確かめ、メモに書く。符号の約束 (`EyeBallX`・`MouthX`・`JawX`) を確かめられたら T0306 の未決の解消に使う
- [ ] Tauri でカメラを使うための設定 (macOS の `NSCameraUsageDescription`、Linux の WebKitGTK・Windows の WebView2 での許可) を決めて書き、確かめられなかった OS は利用者の確認項目にする
- [ ] `just ci`・`just licenses`・ビューアのビルドが通る

## 進め方

見本のモデル (T0702) は確認にだけ使うので、T0702 を待たずに着手する (オーケストレーターの判断)。T0702 が main に入るまでは `ugoe_model::fixtures::avatar_profile` などで確かめ、入ったら見本のモデルで確かめ直す。

## 参考

- https://ai.google.dev/edge/mediapipe/solutions/vision/face_landmarker/web_js
- `docs/spec/avatar-profile.md#フェイストラッキングとの対応`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
