---
id: T0104
title: デフォーマ・パーツ階層・グルーの評価
status: done
milestone: M1
crates: [ugoe-runtime, ugoe-model (fixtures の追加のみ)]
depends_on: [T0102]
assignee: claude-worker-T0104
---

# T0104: デフォーマ・パーツ階層・グルーの評価

## 目的

`CompiledModel` がデフォーマ・パーツ階層・グルーをコンパイルし、`ModelInstance::update` が docs/spec/evaluation.md の手順 2〜7 を実装する。

## 受け入れ条件

- [x] ワープデフォーマ (範囲内と範囲外)、回転デフォーマ (親なし、親が回転、親がワープ) のテストがある
- [x] パーツとデフォーマの不透明度の伝播のテストがある
- [x] グルーのテストがある
- [x] デフォーマを含むフィクスチャを `ugoe_model::fixtures` に追加する
- [x] ワープの範囲外の外挿方式について、比較した結果を仕様書に反映し、`> 未決:` を解消する
- [x] 毎フレームのヒープ確保がない (事前に確保したバッファを再利用する)
- [x] `just ci` が通る

## 参考

- `docs/spec/evaluation.md`

## メモ

- ワープの範囲外は「境界へクランプした点の双線形の値 + 四隅から作ったアフィン近似の軸 × はみ出し量」に決めた。端のセルの延長 (線形外挿) との比較は `crates/ugoe-runtime/src/deform.rs` のテストと `docs/spec/evaluation.md` の表にある (台形の格子で裏返らない、端のセルの局所的な潰れを増幅しない)。
- 毎フレームのヒープ確保がないことは、`instance::tests::update_does_not_reallocate_buffers` で全バッファの先頭アドレスと容量が `update()` の前後で変わらないことで確かめた (独自アロケータは `unsafe` が要るので使わない)。描画順のソートは確保をしない `sort_unstable_by` に (描画順, インデックス) の全順序を与えて、安定ソートと同じ結果にした。
- 追加した公開 API: `CompiledModel::deformer_order()` (親が先の評価順)。既存の公開項目は変えていない (`CompileError::Unsupported` は `rows`/`cols` が 0 のワープの拒否に使う)。
- 追加したフィクスチャ: `warp_grid`、`rotation_chain`、`rotation_under_warp`、`part_opacity_hierarchy`、`glued_quads`。
- 後続の候補:
  - `validate` でワープの `rows`/`cols` が 0 のものを検出する (今はランタイムのコンパイルで拒否。`ugoe-model` の範囲外なので触っていない)。
  - ブレンドシェイプ (evaluation.md 6.) は未実装 (T0303)。今は `blend_shape` のパラメータも通常の軸として補間される。
  - エディタでデフォーマのギズモを描くために、ワープの制御点や回転デフォーマのワールド変換を読む API が要る。
