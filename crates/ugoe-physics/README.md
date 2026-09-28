# ugoe-physics

**層:** L3 機能 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

振り子チェーンによる物理演算 (`PhysicsSystem`)。

- `PhysicsSystem`: パラメータの入出力、固定ステップ、出力の補間。
- `chain::Chain`: 振り子チェーン 1 本の積分。パラメータを知らないので、動的物理 (T0305) からも使う。

## 守ること

- 決定的であること
- 固定ステップで積分する

## 仕様

- [docs/spec/physics.md](../../docs/spec/physics.md)

## 関連タスク

- T0301 (`docs/tasks/T0301-*.md`)
- T0305 (動的物理)
