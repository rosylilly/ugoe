---
id: T0108
title: キーフォーム推定のためのデータ定義
status: done
milestone: M2
crates: [ugoe-model, ugoe-format (テストの追従のみ)]
depends_on: []
assignee: claude-worker-T0108
---

# T0108: キーフォーム推定のためのデータ定義

## 目的

キーフォームのセルの由来 (`authored` / `derived`)、推定の設定、ミラーの対応 (`mirror`)、頂点・制御点の `depth` を `ugoe-model` に追加する (インターフェースを先に入れるタスク)。

## 受け入れ条件

- [x] `KeyformGrid::origins` (セルごとの由来、既定はすべて `authored`) と推定の設定を追加し、serde で往復できる
- [x] `Mesh::mirror` / `Deformer::mirror` と、頂点・制御点の `depth` (オーサリング用、既定なし) を追加する
- [x] `validate` が由来の長さと `mirror` の参照を検査する
- [x] 古い JSON (新しいフィールドなし) が読める
- [x] `docs/spec/model.md` の「予定されているデータ」と `docs/spec/keyform-inference.md` を実装に合わせて更新する
- [x] `just ci` が通る

## 参考

- `docs/adr/0010-keyform-inference.md`
- `docs/spec/keyform-inference.md`

## メモ

- 追加した型: `KeyformOrigin` (`authored` / `derived`)、`KeyformInference` (`method` でタグ付け: `additive` / `depth_rotation`)、`DepthRotation` (`angle_scale` / `pivot` / `pivot_depth` / `yaw` / `pitch`)。
- 追加したフィールド: `KeyformGrid::{origins, inference}`、`Mesh::{mirror, mirror_axis, depth}`、`Deformer::{mirror, mirror_axis}`、`WarpDeformer::depth`。すべて `#[serde(default)]` で、既定値のときは書き出さない (既存モデルの JSON は変わらない)。
- 対称軸は要素ごとの `mirror_axis` (親座標系の x) にした。省略時は親で決まる既定値 (キャンバスの原点 x / ワープ `0.5` / 回転 `0`)。対の 2 要素では元の側の軸を使う。
- `KeyformGrid::new(bindings, keyforms)` を追加し、構造体リテラルを置き換えた。`ugoe-runtime` のテスト 1 か所 (`KeyformGrid` のリテラル) も機械的に追従した (範囲外だがコンパイルに必要)。
- フィクスチャ `keyform_inference()` を追加 (ミラーの対・深度・由来・深度からの回転の設定を持つ)。T0610〜T0612 のテスト入力に使える。
- 後続 (T0610〜T0612) で扱うもの: 加算合成の「中立のキーがない軸」の検査、ミラーの対の種類・形 (ワープの `rows`/`cols`、頂点数) の一致、親が異なる対の扱い。`validate` ではまだ検査していない。
- `Parameter::mirror` の双方向性は検査していない (既存の仕様どおり参照の存在だけ)。
