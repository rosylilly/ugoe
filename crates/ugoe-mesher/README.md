# ugoe-mesher

**層:** T1 素材処理 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

アルファ付き画像から三角形メッシュを自動生成する。

## 守ること

- 決定的であること (同じ画像とオプションからは同じメッシュ)
- 頂点数を u16 の範囲に収める

## 仕様

- [docs/ai-integration.md#自動メッシュ](../../docs/ai-integration.md#自動メッシュ)

## 関連タスク

- T0502 (`docs/tasks/T0502-*.md`)
