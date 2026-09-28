# ugoe-psd

**層:** T1 素材処理 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

PSD から、PSD の形式に依存しない `LayerTree` を作る。

## 守ること

- PSD パーサの実装の詳細をこのクレートの外に漏らさない
- 壊れた入力でパニックしない

## 仕様

- [docs/ai-integration.md#psd-取り込み](../../docs/ai-integration.md#psd-取り込み)

## 関連タスク

- T0501 (`docs/tasks/T0501-*.md`)
