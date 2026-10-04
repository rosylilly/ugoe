---
id: T0542
title: 深度推定サーバへのリクエストの再試行・認証・エラーの種類
status: todo
milestone: M9
crates: [ugoe-ai, docs]
depends_on: [T0527]
assignee:
---

# T0542: 深度推定サーバへのリクエストの再試行・認証・エラーの種類

## 目的

T0527 のレビューの所見。`HttpDepthEstimator` (`crates/ugoe-ai/src/depth.rs`) は、LLM のプロバイダと違って

- 再試行しない (`transport::send_with_retry` を使わない。推論サーバの起動直後の接続の失敗や 503 でそのまま失敗する)
- 認証のヘッダを送れない (外部の推論サーバは `Authorization` を要ることが多い。送り先がループバックでない設定は許しているのに、鍵を渡す手段がない)
- 通信の失敗を `AiError::Network(e.message)` にして、種類 (タイムアウト・接続) を落とす

プロバイダと同じ規則にそろえるか、ループバックだけを想定すると決めて文書に書くかを決める。

## 受け入れ条件

- [ ] 再試行を `RetryPolicy` と同じ規則にする (`HttpDepthConfig` に `retry` を足す) か、しない理由を書く
- [ ] 認証のヘッダ (`ApiKey` を `Authorization: Bearer` で) を任意で送れるようにするか、ループバックだけを想定すると決める。送るなら秘密がエラー・録画・`Debug` に出ないテストを足す
- [ ] 通信の失敗のメッセージに種類を残す (プロバイダの `transport_error` と同じ形)
- [ ] `docs/ai-integration.md#深度推定による振り向きリグ` を更新する
- [ ] `just ci` が通る

## 参考

- docs/ai-integration.md
- crates/ugoe-ai/src/claude.rs (`send_with_retry`・`redact`)

## メモ
