---
id: T0803
title: スタジオ: 入力 (カメラのトラッキング・マイクの口パク・ホットキー)
status: done
milestone: M8
crates: [apps/studio, packages, docs, Cargo.toml, Cargo.lock, pnpm-lock.yaml, .gitignore]
depends_on: [T0802, T0134]
assignee: claude-worker-T0803
---

# T0803: スタジオ: 入力 (カメラのトラッキング・マイクの口パク・ホットキー)

## 目的

配信中に手を使わずにアバターを動かせるようにする。

## 受け入れ条件

- [x] カメラのフェイストラッキング (@ugoe/face-tracking)。モーションの優先 (T0721・T0722) と待機モーション
- [x] マイクの音量から口パク (ランタイムの口パクの入力に渡す)。入力の機器の選択と感度・ノイズの閾値
- [x] グローバルなホットキー (アプリが前面に無くても効く) で、表情・モーション・シーン・着せ替えの切り替え。割り当ての画面と保存
- [x] `just ci` が通る

## 参考

- docs/spec/motion.md、packages/face-tracking

## メモ

- **やったこと:** 入力は指揮役 (バックエンドの `Puppet`) に入れ、受け手には値を配るだけにした (ADR 0028)。
  - 指揮役の演技 (`apps/studio/src-tauri/src/animation.rs`): モーション (優先度)・表情 (set / add / remove / toggle / clear)・待機モーション
    (名前で選ぶ規則はビューアと同じ)・トラッキング中のモーションの優先 (待機の間は効かせない)・トラッキングのままにするパラメータ・
    トラッキングの入力 (外す・一時停止で既定値に戻す)・自動の目パチ・口パクの音量と一時停止。モデルを読み直しても入力と設定は引き継ぐ。
  - 操作ウィンドウ: 「表情とモーション」「入力」(`@ugoe/face-tracking` のパネル + マイクの機器・感度・ノイズの閾値・メーター)「ホットキー」のパネルと、
    中央の「いまの状態」にトラッキング・口パク・モーション・表情。文言は @ugoe/i18n (日英)、部品は @ugoe/ui。
  - 入力の処理は `requestAnimationFrame` に頼らず、バックエンドの刻みのスレッドが送るメッセージ (`subscribe_pulse`) で回す。
    `FaceTracker` に `schedule` の選択肢を足した (`packages/face-tracking`。既定は今までどおり requestAnimationFrame)。
  - マイクの口パク: RMS → dB (−60〜0) → ノイズの閾値と感度 → 開く速さ・閉じる速さ (`src/lib/lipSync.ts`)。
  - ホットキー: `tauri-plugin-global-shortcut` でバックエンドが登録する。操作はシーン・表情・表情をすべて外す・モーション・モーションを止める・
    着せ替え (スロットのモジュールを替える / 空ける)・トラッキングの一時停止とキャリブレーション・口パクの一時停止。割り当ての画面はキーを押して決め、
    押している間は登録を外す。ぶつかり・読めないキー・OS に登録できなかったものは理由を出す。Wayland では知らせを出す。
  - プロファイルに `hotkeys`・`input`・`motion` を省ける項目として足した (版は 1 のまま。docs/studio.md#入力とホットキーの項目-t0803)。
  - カメラとマイクの許可は操作ウィンドウだけ (`on_permission_request`)。macOS の `Info.plist`・`Entitlements.plist` を足した。
- **確かめたこと:** ヘッドレスの Chrome と使い捨てのブリッジ (本物の `Stage`・`Conductor`) で、`avatar.ugoe` のモーション・表情・待機・優先度、
  偽のカメラ (アバターの静止画。アニメの顔なので MediaPipe は顔を見つけない) でカメラが動くこと、記録の再生でトラッキングの値が指揮役に届くこと、
  **`requestAnimationFrame` を止めても値が届き続けること**、偽のマイクで口を開く量が届くこと、ホットキーの割り当て (キーを押して決める・ぶつかり)・
  押したときの操作 (表情の toggle)、一時停止、プロファイルへの保存と開き直し (日本語・ダーク・狭い画面)。
  画面: `scratchpad/T0803/shots/` (01〜08)。
- **確かめていないこと (利用者の確認に残す):** 実際のカメラで顔が動くこと、実際のマイク、OS へのホットキーの登録と、スタジオが前面に無いときに効くこと
  (Windows・macOS・Linux X11)、Wayland の振る舞い、操作ウィンドウを最小化したときにカメラの映像とマイクの音を取り続けるか (WebKitGTK・WebView2・WKWebView)。
- **範囲外の変更:** ルートの `Cargo.toml` (`tauri-plugin-global-shortcut` を `[workspace.dependencies]` に。ADR 0028 の 9. の依存)・`Cargo.lock`・
  `pnpm-lock.yaml` (スタジオが `@ugoe/face-tracking` に依存)・`.gitignore` (`/apps/studio/public/mediapipe/`)。
- **後続の候補:** 受け手 (`Puppet::follow`) がモーションのパーツの不透明度のカーブを使わないので、パーツを切り替えるモーションが出力に出ない (T0133 の範囲)。
  トラッキングが届いたら次の刻みを早める調整 (ADR 0028 の 3. の遅れ) は未実装。スタジオで MediaPipe のライセンスの表記を画面に出す口が無い (ビューアはステータスバー)。
  外部 API (T0805) からホットキーと同じ操作を呼べるようにする (Wayland の代わり)。
- 範囲外の変更 (team-lead が確認): ルートの Cargo.toml と Cargo.lock (ADR 0028 の 9. で決めた tauri-plugin-global-shortcut)、pnpm-lock.yaml、.gitignore (/apps/studio/public/mediapipe/)。
