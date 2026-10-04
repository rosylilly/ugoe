---
id: T0537
title: 推定の格子の検査を検証 (validate) に頼らないようにする
status: done
milestone: M9
crates: [ugoe-rigging, ugoe-editor-core, docs]
depends_on: [T0526]
assignee: claude-worker-T0537
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

- [x] 昇順でない・重複した・有限でないキー、重複した軸の格子がエラーになり、格子を変えないテストがある
- [x] 大きすぎる格子が専用のエラーになる
- [x] エディタのエラーの扱いと仕様書を合わせて直した
- [x] `just ci` が通る

## 参考

- T0526 のメモ (所見の表)
- docs/spec/keyform-inference.md#記法、docs/workflow/code-review.md

## メモ

- やったこと:
  - `InferError` に `DuplicateAxis { parameter }`・`NonFiniteKey { parameter, key }`・`KeysNotAscending { parameter, keys }`
    (降順の箇所と重複したキー。`ugoe_model::ValidationError::KeysNotAscending` と同じ区切り)・`GridTooLarge` を足した。
    `GridSizeMismatch` の `usize::MAX` の流用はやめた。
  - `grid.rs` の `Layout::new` が、軸ごとに パラメータ → 軸の重複 → キーの有限 → 狭義の昇順 → 中立のキー の順に検査する。
    加算合成・深度からの回転・ミラー・親の座標系 (`space.rs`) は、すべて `Layout::new` を通るので同じ検査を受ける。
  - テスト: `tests/additive.rs` の `malformed_keys_and_axes_are_errors` (エラーの種類と、格子を変えないこと)、
    `oversized_and_overlong_grids_are_errors` を `GridTooLarge` に。エディタは `EditError::Inference` が `InferError` を包むだけで
    表示は `to_string` (`edit.inference_failed` の `detail`) なので、コードの変更はなく、`inference_tests.rs` の
    `infer_keyforms_rejects_invalid_targets` に昇順でないキー・重複した軸の拒否 (モデルを変えない) を足した。
  - 仕様: `docs/spec/keyform-inference.md#記法` に推定が確かめる格子の条件とエラーの種類を、`docs/editor.md#推定とミラー` の例に追記。
    推定のアルゴリズムは英語の公開仕様に含めない (`docs/spec/public-spec.md`) ので `spec/` は変えていない。
- やらなかったこと: 空のキーの軸は従来どおり `MissingNeutralKey` (中立のキーがない) のまま。
- 起票の候補: なし。
