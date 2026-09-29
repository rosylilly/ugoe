# ugoe-runtime

**層:** L2 評価 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

パラメータ値から描画可能要素の状態を計算する (`CompiledModel`、`ModelInstance`、`ParameterValues`)。

## 守ること

- 決定的であること
- GPU・I/O・スレッド・時刻取得に依存しない。wasm32 でビルドできること
- `update()` で毎フレームのヒープ確保をしない

## 仕様

- [docs/spec/evaluation.md](../../docs/spec/evaluation.md)
- [docs/spec/modules.md](../../docs/spec/modules.md) (着せ替えの合成、`compose`)

## 関連タスク

- T0104 (`docs/tasks/T0104-*.md`)
- T0106 (`docs/tasks/T0106-*.md`)
