---
id: T0543
title: ugoe-ai の公開 API の小さな整理 (ApiKey の置き場所・待ち時間の表示)
status: done
milestone: M9
crates: [ugoe-ai, ugoe-cli, ugoe-mcp, editor, docs]
depends_on: [T0527]
assignee: claude-worker-T0543
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

- [x] `ApiKey` の置き場所を決めて移し、呼び出し側 (CLI・MCP・エディタ) を直す
- [x] 待ち時間の表示を切り上げにし、テストを足す (ugoe-ai とエディタの両方)
- [x] `AiError` の doc コメントをそろえる
- [x] `crates/ugoe-ai/README.md` と `docs/ai-integration.md` を更新する
- [x] `just ci` が通る

## 参考

- crates/ugoe-ai/src/lib.rs・claude.rs・openai.rs
- apps/editor/src-tauri/src/error.rs

## メモ

- 置き場所: `ApiKey` をクレートの直下 (`ugoe_ai::ApiKey`。定義は非公開のモジュール `src/api_key.rs`) に移した。`claude::ApiKey` の再公開は残していない (ugoe-ai は crates.io に出さず、呼び出し側はすべて同じ PR で直したため。`transport` は HTTP の層で鍵の持ち主ではないので選ばなかった)。
- 環境変数から読む関数を `ApiKey::from_env(name)` の 1 つにした (旧 `from_env()` は Claude 専用の `ANTHROPIC_API_KEY` を共通の型に埋めていたので、`ApiKey::from_env(claude::API_KEY_ENV)` と書く。旧 `from_env_var` は改名)。
- 呼び出し側: エディタ (`ai.rs`・`ai_provider.rs`・`proposals_tests.rs`) と ugoe-ai のテストの import を直した。CLI と MCP は `ApiKey` を使っていなかったので変更なし。
- 待ち時間: `ugoe_ai::secs_rounded_up` を足し、`AiError::RateLimited`・`TimeLimitExceeded` の文言とエディタのエラーの引数 (`retry_after`・`minutes`) をこれで切り上げにそろえた。テストは ugoe-ai (`lib.rs`) とエディタ (`error.rs` の `ai_errors_keep_their_kind`。0.5 秒 → 1 秒、60.5 秒 → 2 分)。
- `AiError::Provider` は `FakeProvider` のほかに、リクエストを送るスレッドを作れないとき (`transport.rs`) にも使っていたので残し、doc コメントで用途を書いた。`AiError` 自身と、doc の無かったヴァリアント・フィールド (`Provider`・`Api` の `status`/`message`・`RateLimited`・`Refused`・`TimeLimitExceeded` のフィールド) に doc コメントを付けた。
- 文書: `crates/ugoe-ai/README.md`、`docs/ai-integration.md` (Claude の節にエラーの項、API キーの節)、`docs/editor.md` (キーの渡し先、`ai.rate_limited`・`ai.time_limit` の引数が切り上げであること)。
- `just ci` は通った。

