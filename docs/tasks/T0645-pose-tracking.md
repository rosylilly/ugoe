---
id: T0645
title: トラッキング: MediaPipe の Pose Landmarker で体の向きと肩を取り込む
status: in-progress
milestone: M4
crates: [packages, apps/viewer, apps/editor, ugoe-motion, docs, spec]
depends_on: [T0636, T0703]
assignee: claude-worker-T0645
---

# T0645: トラッキング: MediaPipe の Pose Landmarker で体の向きと肩を取り込む

## 目的

利用者の要望。顔のトラッキング (Face Landmarker) に Pose Landmarker を組み合わせ、体の向き (`BodyAngleX/Y/Z`) と肩の上下を取り込む。
アバタープロファイルの仕様は「体の向きは、体のトラッキングがあればその回転、なければ頭の回転 × 0.3」としており、その「体のトラッキング」を作る。

## 受け入れ条件

- [ ] `packages/face-tracking` に Pose Landmarker を足す (モデルはビルドのときに取得して SHA-256 を検証。リポジトリにはコミットしない。既定は軽いモデル)。顔と体の推定を同じフレームで行い、体のトラッキングは画面で切れる
- [ ] 体の点から `BodyAngleX` (左右の回転)・`BodyAngleY` (前後の傾き)・`BodyAngleZ` (肩の線の傾き) を求める式と、肩の上下 (すくめ) のパラメータ。肩のパラメータはアバタープロファイルの拡張のパラメータとして定める (ID・値域・意味)。頭の角度から体の回転を引くかどうか (体ごと回ったときに頭が二重に回らないように) を決める
- [ ] 肩が映っていない・信頼度が低いときは、今の「頭の回転 × 0.3」に戻る。平滑化・キャリブレーション・記録と再生 (T0703) に体の値も含める
- [ ] アバタープロファイルの仕様 (`docs/spec/avatar-profile.md` と英語の `spec/avatar-profile.md`) のトラッキングの入力・式・拡張のパラメータを更新する (`just spec` が通る)。Rust 側の対応 (ugoe-motion) に体の入力が要るなら足す
- [ ] ビューアとエディタのプレビューで動く。記録した値の列でのテスト、ヘッドレスのブラウザでの確認。操作マニュアルとサードパーティのライセンスの表記を更新する
- [ ] `just ci`・`just licenses`・エディタとビューアのビルドが通る

## 参考

- `docs/tasks/T0703-viewer-face-tracking.md`・`docs/tasks/T0636-editor-preview-tracking.md` のメモ
- `docs/spec/avatar-profile.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
