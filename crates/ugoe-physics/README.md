# ugoe-physics

**層:** L3 機能 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

振り子チェーンによる物理演算 (`PhysicsSystem`)。

- `PhysicsSystem`: パラメータの入出力、固定ステップ、出力の補間。
- `chain::Chain`: 振り子チェーン 1 本の積分。パラメータを知らないので、動的物理からも使う。
- `collider::WorldCollider`・`collider::constrain_segment`: 衝突体 (円・カプセル)。区間の長さを保って親のまわりに回し、
  衝突体の手前で止める (すり抜けない)。`PhysicsSystem::rest_pose_collisions` で静止姿勢の当たりを調べる。
- 動的物理 (`PhysicsSystem::update_instance`): モデルの 1 段目の評価から根元・衝突体・重力の基準を求め、
  回転デフォーマの角度・原点、ワープの制御点、メッシュの頂点に出力する (2 段階の評価)。

## 守ること

- 決定的であること
- 固定ステップで積分する

## 仕様

- [docs/spec/physics.md](../../docs/spec/physics.md)

## 関連タスク

- T0301 (`docs/tasks/T0301-*.md`)
- T0305 (動的物理)
