# ugoe-rigging

**層:** T1 リギング (依存してよいクレートは `scripts/check-layers.mjs` を参照)

キーフォームの推定を行う。推定の結果はふつうのキーフォームとして格子に焼き込み、ランタイムは推定を知らない。

- 加算合成 (`infer_additive`、`infer_additive_target`)
- ミラーと深度からの回転は T0612 で追加する

## 守ること

- 決定的であること (同じモデルと格子からは同じ結果)
- `authored` のセルを上書きしない。書き込むのは `derived` のセルだけ
- モデルの型は `ugoe-model` に置き、このクレートはアルゴリズムだけを持つ
- 編集は `ugoe_editor_core::Command` から呼ぶ。このクレートは Undo を知らない

## 仕様

- [docs/spec/keyform-inference.md](../../docs/spec/keyform-inference.md)
- [docs/adr/0010-keyform-inference.md](../../docs/adr/0010-keyform-inference.md)

## 関連タスク

- T0610 (`docs/tasks/T0610-*.md`)
- T0611 (エディタのコマンド)、T0612 (ミラーと深度からの回転)
