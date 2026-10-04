---
id: T0319
title: 物理の毎フレームの確保とパラメータの名前の探索を減らす
status: todo
milestone: M9
crates: [ugoe-physics]
depends_on: [T0316]
assignee:
---

# T0319: 物理の毎フレームの確保とパラメータの名前の探索を減らす

## 目的

`PhysicsSystem::update` / `update_instance` は毎フレーム次を行っている (T0316 のレビューで見つけた)。

- `RigFrame` の `Vec` (リグごとに `references`・`flips`)、`targets`・`starts` の `Vec` を作る。ステップごと・リグごとに `outputs()` が `Vec` を作る。
- 中継があると `Relay::new` が `ParameterValues` を丸ごと複製する (パラメータの ID の `String` もすべて複製する)。
- 入力 (`root_pose`、ステップごと) と出力 (`write_parameters`・`Relay::new`) のたびに、`ParameterValues::index_of` でパラメータの名前を線形に探す
  (出力の数 × パラメータの数の文字列比較)。

ランタイムの目標 (1 体 1ms、roadmap.md の M7) に対して今すぐ問題になる量ではないが、物理の計測がないので、計測を足してから減らす。

## 受け入れ条件

- [ ] 物理の 1 フレームの時間を測るベンチマーク (またはテスト内の計測) を足す (`fixtures::dynamic_hair`・`physics_relay`)
- [ ] パラメータの添字を解決して持つ (渡された `ParameterValues` の並びが変わったら解決し直す)、写しと出力のバッファを使い回すなどで、毎フレームの確保と名前の探索をなくす
- [ ] 結果がビット単位で変わらないことを、既存の決定性のテストと前後の比較で確かめる
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- `docs/tasks/T0316-review-physics.md` のメモ
- `docs/spec/physics.md`

## メモ
