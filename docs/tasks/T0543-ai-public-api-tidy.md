---
id: T0543
title: ugoe-ai の公開 API の小さな整理 (ApiKey の置き場所・待ち時間の表示)
status: todo
milestone: M9
crates: [ugoe-ai, ugoe-cli, ugoe-mcp, editor, docs]
depends_on: [T0527]
assignee:
---

# T0543: ugoe-ai の公開 API の小さな整理 (ApiKey の置き場所・待ち時間の表示)

## 目的

T0527 のレビューの所見のうち、公開 API に触れるので T0527 では直さなかった小さなもの。

- `ApiKey` は Claude と OpenAI 互換のプロバイダで共通なのに `ugoe_ai::claude::ApiKey` にある。`ApiKey::from_env_var` も OpenAI 互換のために足したもの。
  クレートの直下 (`ugoe_ai::ApiKey`) か `transport` に移し、`claude::ApiKey` は再公開で残すか決める。
- `AiError::RateLimited` の表示は `retry_after.as_secs()` で切り捨てるので、0.5 秒は「retry after 0s」になる。エディタ (`apps/editor/src-tauri/src/error.rs`) も `as_secs()` を使う。切り上げにそろえる。
- `AiError::Provider` は `FakeProvider` が尽きたときにしか使われない。残すか、doc コメントで用途を書く。
- `AiError` の各ヴァリアントに doc コメントがないもの (`Provider`・`InvalidProposal`) がある。

## 受け入れ条件

- [ ] `ApiKey` の置き場所を決めて移し、呼び出し側 (CLI・MCP・エディタ) を直す
- [ ] 待ち時間の表示を切り上げにし、テストを足す (ugoe-ai とエディタの両方)
- [ ] `AiError` の doc コメントをそろえる
- [ ] `crates/ugoe-ai/README.md` と `docs/ai-integration.md` を更新する
- [ ] `just ci` が通る

## 参考

- crates/ugoe-ai/src/lib.rs・claude.rs・openai.rs
- apps/editor/src-tauri/src/error.rs

## メモ
