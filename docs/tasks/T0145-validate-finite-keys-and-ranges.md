---
id: T0145
title: 検証で、キーフォームの軸のキーとパラメータの値域に有限値を求める
status: done
milestone: M9
crates: [ugoe-model, docs]
depends_on: [T0137]
assignee:
---

# T0145: 検証で、キーフォームの軸のキーとパラメータの値域に有限値を求める

## 目的

T0137 (ugoe-runtime のコードレビュー) で見つけた検証の穴を埋める。

`ugoe_model::validate` は、軸のキーが狭義の昇順であること (NaN を含まないこと) と、パラメータの `min <= default <= max` を確かめるが、
**無限大は通す**。MessagePack (`.ugoe`) は無限大を表せるので、次のモデルが検証を通る。

- キーに `-inf` / `inf` を含む軸: 区間の幅が無限大になり、補間の係数 `t` や曲線補間の接線が NaN になる。
  NaN の重みは取り除かれるので、その格子の補間結果がすべて 0 (頂点が原点に潰れる) になる。パニックはしない。
- 値域が無限大のパラメータ: クランプが効かず、無限大の値がそのまま評価に入る。

公開仕様 (`spec/model.md#data-conventions`) は「特に断らない `f32` は有限値でなければならない (MUST)。検証で確かめるのは列挙したフィールドだけで、
ほかの有限でない値は読み手が拒否してよい (MAY)」としている。ランタイムの結果は仕様の範囲 (定義の計算でも有限でない) だが、
作者の誤りを早く見つけるために、検証の規則に加えるかを決めて実装する。

## 受け入れ条件

- [ ] キーフォームの軸のキー (ブレンドシェイプ格子を含む) と、パラメータの `min`・`default`・`max` を有限値に限るかを決める (加えるなら `InvalidValue` などで報告し、テストを足す)
- [ ] キーフォームの数値 (頂点位置・制御点・角度・不透明度など) も同じ扱いにするかを決め、決めたことを `docs/spec/model.md#検証` と `spec/model.md#model-validation` に書く
- [ ] 既存のフィクスチャ・見本・互換性テストの資料 (`assets/compat/`) が検証を通ることを確かめる
- [ ] `just ci` が通る

## 参考

- docs/tasks/T0137-review-runtime.md (メモの所見の表)
- docs/spec/model.md#検証、spec/model.md#data-conventions、spec/model.md#model-validation
- spec/evaluation.md#conformance (有限でない結果は許容誤差の対象外)

## メモ

- **閉じた (T0008):** T0141 にまとめた。同じ「検証で数値の有限性を確かめる」の判断で、T0141 の受け入れ条件にキーフォームの軸のキーとパラメータの値域を足した。
