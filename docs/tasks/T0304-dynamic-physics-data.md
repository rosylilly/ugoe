---
id: T0304
title: 動的物理のデータ定義
status: todo
milestone: M3
crates: [ugoe-model]
depends_on: []
assignee:
---

# T0304: 動的物理のデータ定義

## 目的

物理の出力先 (`deformer_angle` / `deformer_offset` / `vertices`)、衝突体 (円・カプセル)、重力の基準のデフォーマ、粒子の `collision_radius` を定義する (インターフェースを先に入れるタスク)。

## 受け入れ条件

- [ ] 型を定義し、serde で往復できる
- [ ] `validate` が出力先・衝突体の取り付け先の参照を検査する
- [ ] 古いデータ (パラメータへの出力だけ) が読める
- [ ] `docs/spec/physics.md` と `docs/spec/model.md` を更新する
- [ ] `just ci` が通る

## 参考

- `docs/adr/0011-dynamic-physics.md`
- `docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
