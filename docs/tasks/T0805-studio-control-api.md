---
id: T0805
title: スタジオ: 外から操作する API (ループバックの WebSocket・トークン・利用者の許可)
status: in-progress
milestone: M8
crates: [apps/studio, docs]
depends_on: [T0803]
assignee: claude-worker-T0805
---

# T0805: スタジオ: 外から操作する API (ループバックの WebSocket・トークン・利用者の許可)

## 目的

Stream Deck やチャットのボットなど、外のツールから表情やシーンを切り替えられるようにする。

## 受け入れ条件

- [ ] ループバックだけで待つ WebSocket の API。初回の接続は利用者の許可 (操作ウィンドウで承認) とトークン。既定は切
- [ ] 操作: 表情・モーション・シーンの一覧と切り替え、パラメータの値の設定、着せ替え。イベントの通知 (シーンの変化など)
- [ ] メッセージの JSON Schema と docs (英語の API リファレンス)、接続と操作のテスト
- [ ] `just ci` が通る

## 参考

- ADR 0028 (安全性)、crates/ugoe-mcp の --http の扱い

## メモ
