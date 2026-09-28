---
id: T0104
title: デフォーマ・パーツ階層・グルーの評価
status: in-progress
milestone: M1
crates: [ugoe-runtime, ugoe-model (fixtures の追加のみ)]
depends_on: [T0102]
assignee: claude-worker-T0104
---

# T0104: デフォーマ・パーツ階層・グルーの評価

## 目的

`CompiledModel` がデフォーマ・パーツ階層・グルーをコンパイルし、`ModelInstance::update` が docs/spec/evaluation.md の手順 2〜7 を実装する。

## 受け入れ条件

- [ ] ワープデフォーマ (範囲内と範囲外)、回転デフォーマ (親なし、親が回転、親がワープ) のテストがある
- [ ] パーツとデフォーマの不透明度の伝播のテストがある
- [ ] グルーのテストがある
- [ ] デフォーマを含むフィクスチャを `ugoe_model::fixtures` に追加する
- [ ] ワープの範囲外の外挿方式について、比較した結果を仕様書に反映し、`> 未決:` を解消する
- [ ] 毎フレームのヒープ確保がない (事前に確保したバッファを再利用する)
- [ ] `just ci` が通る

## 参考

- `docs/spec/evaluation.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
