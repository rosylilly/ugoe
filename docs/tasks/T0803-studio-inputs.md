---
id: T0803
title: スタジオ: 入力 (カメラのトラッキング・マイクの口パク・ホットキー)
status: in-progress
milestone: M8
crates: [apps/studio, packages, docs]
depends_on: [T0802, T0134]
assignee: claude-worker-T0803
---

# T0803: スタジオ: 入力 (カメラのトラッキング・マイクの口パク・ホットキー)

## 目的

配信中に手を使わずにアバターを動かせるようにする。

## 受け入れ条件

- [ ] カメラのフェイストラッキング (@ugoe/face-tracking)。モーションの優先 (T0721・T0722) と待機モーション
- [ ] マイクの音量から口パク (ランタイムの口パクの入力に渡す)。入力の機器の選択と感度・ノイズの閾値
- [ ] グローバルなホットキー (アプリが前面に無くても効く) で、表情・モーション・シーン・着せ替えの切り替え。割り当ての画面と保存
- [ ] `just ci` が通る

## 参考

- docs/spec/motion.md、packages/face-tracking

## メモ
