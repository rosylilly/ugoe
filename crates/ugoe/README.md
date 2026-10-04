# ugoe

**層:** L4 SDK (依存してよいクレートは `scripts/check-layers.mjs` を参照)

ランタイム SDK のファサード。`Puppet` が 1 体分の更新 (モーション → 表情 → 目パチ・呼吸 → 物理 → リップシンク → ポーズ → 評価。順序は docs/spec/motion.md#フレーム内の適用順) をまとめる。下位のクレートを再エクスポートする。

## 守ること

- アプリやエンジンの組み込みで最初に使うクレートなので、API を小さく分かりやすく保つ

## 仕様

- [docs/spec/motion.md#フレーム内の適用順](../../docs/spec/motion.md#フレーム内の適用順)
- [docs/spec/motion.md#受け手として進める](../../docs/spec/motion.md#受け手として進める) (`Puppet::set_base_values` / `follow` / `values_before_physics` / `lip_sync_applied` / `part_opacities_applied`。`src/follow.rs`)
- [docs/spec/evaluation.md#9-当たり判定](../../docs/spec/evaluation.md#9-当たり判定) (`Puppet::hit_areas_at` / `hit_test`)
- [docs/spec/modules.md#puppet-での着せ替え](../../docs/spec/modules.md#puppet-での着せ替え) (`Puppet::attach_module` / `detach_module` / `set_modules` / `set_base_physics`。`src/dress_up.rs`)
- 着せ替えの失敗と読み込みの警告の種類 (`DressUpError::code`・`fields`、`LoadWarning`。`src/warning.rs`)。wasm と C ABI が同じ値を返すために
  ここで決める。値は組み込み先が照らし合わせるので変えない (T0412)
- [docs/spec/file-formats.md#読み込みの上限](../../docs/spec/file-formats.md#読み込みの上限) (`limits`: 上限は `ugoe::format::read_bundle_with` /
  `ugoe::format::module_file::read_module_with` に `ReadLimits` を渡して指定する。超えたときの種類と補足の値 `ReadLimitExceeded` は
  wasm と C ABI が同じ値を返すためにここで決める。T0152)
- [docs/portability.md#ホストから来る数](../../docs/portability.md#ホストから来る数) (NaN・無限大・負の `dt` の扱い。前提はこのクレートと
  `ugoe-runtime` で決め、C ABI・wasm・Godot・Unity は表し方だけを持つ。`tests/host_numbers.rs`)
- [docs/spec/rendering.md#ホストでのライティング](../../docs/spec/rendering.md#ホストでのライティング) (`lighting`: `uses_lighting` / `Puppet::uses_lighting`・`bundle_normal_maps`)

## 関連タスク

- T0303 (`docs/tasks/T0303-*.md`)
- T0308 (当たり判定)
- T0121 (着せ替え)
- T0210 (ライティングの補助)
- T0412 (エラーと警告の code)
- T0133 (受け手として進める)
- T0134 (受け手の段の順を update とそろえる)
- T0135 (受け手でもモーションのパーツの不透明度のカーブを当てる)
- T0152 (読み込みの上限)
