---
id: T0318
title: 物理の検証で、係数・重力・重みが有限値かを確かめる
status: done
milestone: M9
crates: [ugoe-model, docs]
depends_on: [T0316]
assignee:
---

# T0318: 物理の検証で、係数・重力・重みが有限値かを確かめる

## 目的

`validate_physics` (`crates/ugoe-model/src/validate/physics.rs`) は参照先と `collision_radius`・衝突体の値しか検査しない。
重力・風・粒子の係数 (`position`・`radius`・`mobility`・`delay`・`acceleration`)・入力の `weight`・出力の `scale`・`weight`・
正規化の範囲が無限大や NaN でも読み込みが通る (JSON の `1e39` は f32 で無限大になる)。
T0316 で確かめたところ、`gravity` が NaN・粒子の `radius` が無限大・出力の `scale` が無限大の設定では、
物理はパニックしないが、動的物理の出力を通してメッシュの頂点が NaN になり、レンダラに渡る。
入力の `weight` が無限大のリグは、鎖の粒子が NaN のまま戻らない (リセットまで止まる)。

## 受け入れ条件

- [ ] `validate_physics` が次を検査する (エラーの形は既存の `InvalidValue`): `gravity`・`wind` の成分が有限、`fps` が有限
      (0 以下は既定の 60 を使うので許す)、粒子の `position` が有限・`radius`・`delay`・`acceleration` が 0 以上の有限値・
      `mobility` が有限 (1 を超えると発散するので 0〜1 にするかは決めて書く)、入力の `weight`・出力の `scale`・`weight`・正規化の範囲が有限
- [ ] `docs/spec/model.md#物理の検証` と英語の公開仕様の対応する節を同じ PR で直す
- [ ] 既存のフィクスチャと `assets/compat/` のファイルが検査を通る
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0316-review-physics.md` のメモ
- `docs/spec/model.md#物理の検証`、`docs/spec/physics.md`

## メモ

- **閉じた (T0008):** T0141 にまとめた。同じ「検証で数値の有限性を確かめる」の判断で、T0141 の受け入れ条件に物理の係数・重力・重み・`mobility` の範囲を足した。
