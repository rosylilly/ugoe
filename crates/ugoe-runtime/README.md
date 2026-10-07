# ugoe-runtime

**層:** L2 評価 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

パラメータ値から描画可能要素の状態を計算する (`CompiledModel`、`ModelInstance`、`ParameterValues`)。

## 守ること

- `no_std` + `alloc` で書く。既定で有効な feature `std` を外しても公開 API は変わらない (`just nostd-check`。[規則](../../docs/portability.md#ランタイム層が守る制約-移植性のため))
- 決定的であること
- GPU・I/O・スレッド・時刻取得に依存しない。wasm32 でビルドできること
- `update()` で毎フレームのヒープ確保をしない
- ID から添字への引き当て (`ParameterValues::index_of`・`CompiledModel::*_index`) を線形探索にしない (毎フレームの経路で呼ばれる。T0146、`examples/id_lookup_timing.rs` で測る)。表は `BTreeMap` (O(log n)。`std` の `HashMap` は no_std に無く、決まったハッシュ関数ではファイルから来る ID で衝突を狙われうるため。T0181)

## 仕様

- [docs/spec/evaluation.md](../../docs/spec/evaluation.md)
- [docs/spec/modules.md](../../docs/spec/modules.md) (着せ替えの合成、`compose`)
- [docs/portability.md#ホストから来る数](../../docs/portability.md#ホストから来る数) (ホストが書く値の NaN・無限大: `ParameterValues::set`・
  `set_part_opacity_override`・`DynamicOffsets`)

## 関連タスク

- T0104 (`docs/tasks/T0104-*.md`)
- T0106 (`docs/tasks/T0106-*.md`)
