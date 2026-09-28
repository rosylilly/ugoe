# ugoe

**層:** L4 SDK (依存してよいクレートは `scripts/check-layers.mjs` を参照)

ランタイム SDK のファサード。`Puppet` が 1 体分の更新 (モーション → 物理 → 評価) をまとめる。下位のクレートを再エクスポートする。

## 守ること

- アプリやエンジンの組み込みで最初に使うクレートなので、API を小さく分かりやすく保つ

## 仕様

- [docs/spec/motion.md#フレーム内の適用順](../../docs/spec/motion.md#フレーム内の適用順)
- [docs/spec/evaluation.md#9-当たり判定](../../docs/spec/evaluation.md#9-当たり判定) (`Puppet::hit_areas_at` / `hit_test`)

## 関連タスク

- T0303 (`docs/tasks/T0303-*.md`)
- T0308 (当たり判定)
