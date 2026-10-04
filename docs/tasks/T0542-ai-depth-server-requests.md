---
id: T0542
title: 深度推定サーバへのリクエストの再試行・認証・エラーの種類
status: done
milestone: M9
crates: [ugoe-ai, docs]
depends_on: [T0527]
assignee: claude-worker-T0542
---

# T0542: 深度推定サーバへのリクエストの再試行・認証・エラーの種類

## 目的

T0527 のレビューの所見。`HttpDepthEstimator` (`crates/ugoe-ai/src/depth.rs`) は、LLM のプロバイダと違って

- 再試行しない (`transport::send_with_retry` を使わない。推論サーバの起動直後の接続の失敗や 503 でそのまま失敗する)
- 認証のヘッダを送れない (外部の推論サーバは `Authorization` を要ることが多い。送り先がループバックでない設定は許しているのに、鍵を渡す手段がない)
- 通信の失敗を `AiError::Network(e.message)` にして、種類 (タイムアウト・接続) を落とす

プロバイダと同じ規則にそろえるか、ループバックだけを想定すると決めて文書に書くかを決める。

## 受け入れ条件

- [x] 再試行を `RetryPolicy` と同じ規則にする (`HttpDepthConfig` に `retry` を足す) か、しない理由を書く
- [x] 認証のヘッダ (`ApiKey` を `Authorization: Bearer` で) を任意で送れるようにするか、ループバックだけを想定すると決める。送るなら秘密がエラー・録画・`Debug` に出ないテストを足す
- [x] 通信の失敗のメッセージに種類を残す (プロバイダの `transport_error` と同じ形)
- [x] `docs/ai-integration.md#深度推定による振り向きリグ` を更新する
- [x] `just ci` が通る

## 参考

- docs/ai-integration.md
- crates/ugoe-ai/src/claude.rs (`send_with_retry`・`redact`)

## メモ

- プロバイダと同じ規則にそろえた (ループバックだけを想定する案は採らなかった。送り先が外部でもよい設定と `Disclosure` の外部扱いがすでにあり、鍵を渡す手段がないと外部のサーバを使えないため)。
- 再試行: `HttpDepthConfig::retry` (`RetryPolicy`、既定はプロバイダと同じ) を足し、`transport::send_with_retry` で送る。接続の失敗・タイムアウト・408・409・429・5xx を再試行し、`retry-after` に従う。待つ関数は `HttpDepthEstimator::with_sleeper` で差し替えられる。
- 認証: `HttpDepthEstimator::with_api_key(ApiKey)` で `Authorization: Bearer` を送る (省けば送らない。`new` と `with_transport` の形は変えていない)。鍵は `Debug` (`ApiKey(<redacted>)`)・エラーの文言 (`redact`)・受け取ったリクエストの記録・録画したリクエストに出ないことをテストで確かめた。
- エラー: 通信の失敗は `AiError::Network(TransportError の to_string)` (`Timeout: ...` / `Connect: ...`) でプロバイダの `transport_error` と同じ形。429 は `AiError::RateLimited`、それ以外の状態は従来どおり `AiError::Api { kind: "depth_server_error" }` (本文は 500 文字に切り、鍵を伏せる)。
- テスト (`tests/depth_turn.rs`): 接続の失敗と 503 (`retry-after`) の再試行、上限で諦めて種類を残す、4xx は再試行しない、429 はレート制限、鍵を Bearer で送り漏らさない、鍵がなければヘッダを送らない。既存の 503 のテストは `max_retries: 0` にした。
- 文書: `docs/ai-integration.md#深度推定器` の「推論サーバとの約束」 に再試行・エラー・認証を書いた。`crates/ugoe-ai/README.md` の `depth.rs` の行にも一言足した。
- `ApiKey` は `claude` モジュールのものを使った (共通の場所への移動は T0543)。
- 実際の推論サーバ (Depth Anything を載せたもの) では確かめていない (録画の再生だけ)。
- `just ci` は通った。
- 起票の候補: なし
