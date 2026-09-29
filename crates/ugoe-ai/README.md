# ugoe-ai

**層:** T3 自動化 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

AI プロバイダの抽象 (`Provider`) と、自動リギングのパイプライン。

## 守ること

- モデルを直接変更せず、`Proposal` (Command の列) を返す
- CI で実際の API を呼ばない (`FakeProvider` か、録画を再生する `transport::ReplayTransport` を使う)
- API キーをログ・エラー・録画に出さない

## 構成

- `lib.rs`: `Provider` トレイト、`ProviderRequest`、`Proposal`、`Disclosure` (外部に送る内容の要約)
- `claude.rs`: Claude API のプロバイダ (`ClaudeProvider`・`ClaudeConfig`・`ApiKey`)
- `transport.rs`: HTTP の差し替え (`UreqTransport`・`ReplayTransport`・`RecordingTransport`・`Cassette`)
- `fake.rs`: テスト用の `FakeProvider`

## 仕様

- [docs/ai-integration.md](../../docs/ai-integration.md)
- [docs/adr/0006-ai-proposals.md](../../docs/adr/0006-ai-proposals.md)

## 関連タスク

- T0503 (`docs/tasks/T0503-*.md`)
- T0504 (`docs/tasks/T0504-*.md`)
- T0509 (`docs/tasks/T0509-claude-provider.md`)
