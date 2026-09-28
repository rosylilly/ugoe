# ugoe-ai

**層:** T3 自動化 (依存してよいクレートは `scripts/check-layers.py` を参照)

AI プロバイダの抽象 (`Provider`) と、自動リギングのパイプライン。

## 守ること

- モデルを直接変更せず、`Proposal` (Command の列) を返す
- CI で実際の API を呼ばない (`FakeProvider` を使う)

## 仕様

- [docs/ai-integration.md](../../docs/ai-integration.md)
- [docs/adr/0006-ai-proposals.md](../../docs/adr/0006-ai-proposals.md)

## 関連タスク

- T0503 (`docs/tasks/T0503-*.md`)
- T0504 (`docs/tasks/T0504-*.md`)
