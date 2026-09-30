# ugoe

**層:** L4 SDK (依存してよいクレートは `scripts/check-layers.mjs` を参照)

ランタイム SDK のファサード。`Puppet` が 1 体分の更新 (モーション → 表情 → 目パチ・呼吸 → 物理 → リップシンク → ポーズ → 評価。順序は docs/spec/motion.md#フレーム内の適用順) をまとめる。下位のクレートを再エクスポートする。

## 守ること

- アプリやエンジンの組み込みで最初に使うクレートなので、API を小さく分かりやすく保つ

## 仕様

- [docs/spec/motion.md#フレーム内の適用順](../../docs/spec/motion.md#フレーム内の適用順)
- [docs/spec/evaluation.md#9-当たり判定](../../docs/spec/evaluation.md#9-当たり判定) (`Puppet::hit_areas_at` / `hit_test`)
- [docs/spec/modules.md#puppet-での着せ替え](../../docs/spec/modules.md#puppet-での着せ替え) (`Puppet::attach_module` / `detach_module` / `set_modules`。`src/dress_up.rs`)
- 着せ替えの失敗と読み込みの警告の種類 (`DressUpError::code`・`fields`、`LoadWarning`。`src/warning.rs`)。wasm と C ABI が同じ値を返すために
  ここで決める。値は組み込み先が照らし合わせるので変えない (T0412)
- [docs/spec/rendering.md#ホストでのライティング](../../docs/spec/rendering.md#ホストでのライティング) (`lighting`: `uses_lighting` / `Puppet::uses_lighting`・`bundle_normal_maps`)

## 関連タスク

- T0303 (`docs/tasks/T0303-*.md`)
- T0308 (当たり判定)
- T0121 (着せ替え)
- T0210 (ライティングの補助)
- T0412 (エラーと警告の code)
