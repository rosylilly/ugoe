---
id: T0537
title: 推定の格子の検査を検証 (validate) に頼らないようにする
status: todo
milestone: M9
crates: [ugoe-rigging, ugoe-editor-core, docs]
depends_on: [T0526]
assignee:
---

# T0537: 推定の格子の検査を検証 (validate) に頼らないようにする

## 目的

T0526 (ugoe-rigging のコードレビュー) で見つかった、キーフォームの推定の格子の検査 (`crates/ugoe-rigging/src/grid.rs` の `Layout::new`) の
穴を、専用のエラーで塞ぐ。`InferError` に種類を足す公開 API の変更なので、レビューの中では直さなかった。

1. **キーの並びを検査しない (低):** `Layout::new` は軸のキーが昇順で重複がなく有限であることを確かめない。
   `ugoe_model::validate` を通ったモデルでは成り立つが、検証の前のモデル (読み込み途中・AI の提案の組み立て中) で呼ぶと、
   加算合成の 1 軸の補間 (`additive.rs` の `line`) が添字の順を値の順とみなして誤った値を作り、重複したキーでは `0 / 0` で NaN になる。
   同じパラメータの軸が 2 本ある格子も検査しない。
2. **大きすぎる格子のエラーが流用 (低):** T0526 で、セルの数 (軸のキー数の積) が `usize` に収まらない格子をパニックではなく
   `GridSizeMismatch { expected: usize::MAX, .. }` にした。専用の種類 (`GridTooLarge` など) のほうが分かりやすい。

`InferError` の種類を足し、エディタ (`ugoe-editor-core` の推定のエラーの表示・テスト) と `docs/spec/keyform-inference.md` を合わせる。

## 受け入れ条件

- [ ] 昇順でない・重複した・有限でないキー、重複した軸の格子がエラーになり、格子を変えないテストがある
- [ ] 大きすぎる格子が専用のエラーになる
- [ ] エディタのエラーの扱いと仕様書を合わせて直した
- [ ] `just ci` が通る

## 参考

- T0526 のメモ (所見の表)
- docs/spec/keyform-inference.md#記法、docs/workflow/code-review.md

## メモ
