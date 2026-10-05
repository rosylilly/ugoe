# ugoe

**層:** L4 SDK (依存してよいクレートは `scripts/check-layers.mjs` を参照)

ランタイム SDK のファサード。`Puppet` が 1 体分の更新 (モーション → 表情 → 目パチ・呼吸 → 物理 → リップシンク → ポーズ → 評価。順序は docs/spec/motion.md#フレーム内の適用順) をまとめる。下位のクレートを再エクスポートする。

## feature

- `render` (既定で有効): 参照レンダラ (`ugoe-render`、wgpu) を `ugoe::render` として再エクスポートする。エンジン独自のレンダラで描く
  組み込み先は `default-features = false` で外すと wgpu をビルドしない ([docs/portability.md#ugoe-クレートの-feature](../../docs/portability.md#ugoe-クレートの-feature)、T0156)。
  外したビルドの検査は `just sdk-no-render-check`

## 守ること

- アプリやエンジンの組み込みで最初に使うクレートなので、API を小さく分かりやすく保つ
- `ugoe-render` は `render` feature の下だけで使う。`render` の無いビルドでも通るように、`ugoe::render` の型を
  `render` の外の公開 API に出さない (doc コメントのリンクも `ugoe_render` へ張らない)

## 仕様

- [docs/spec/motion.md#フレーム内の適用順](../../docs/spec/motion.md#フレーム内の適用順)
- [docs/spec/motion.md#受け手として進める](../../docs/spec/motion.md#受け手として進める) (`Puppet::set_base_values` / `follow` / `values_before_physics` / `lip_sync_applied` / `part_opacities_applied`。`src/follow.rs`)
- [docs/spec/evaluation.md#9-当たり判定](../../docs/spec/evaluation.md#9-当たり判定) (`Puppet::hit_areas_at` / `hit_test`)
- [docs/spec/modules.md#puppet-での着せ替え](../../docs/spec/modules.md#puppet-での着せ替え) (`Puppet::attach_module` / `detach_module` / `set_modules` / `set_base_physics`。`src/dress_up.rs`)
- 着せ替えの失敗と読み込みの警告の種類 (`DressUpError::code`・`fields`、`LoadWarning`。`src/warning.rs`)。wasm と C ABI が同じ値を返すために
  ここで決める。値は組み込み先が照らし合わせるので変えない (T0412)
- モデルの読み込みの失敗の種類 (`LoadError::code`・`fields`。`src/load_error.rs`)。`.ugoe`・JSON を読めない (`FormatError`) かコンパイルできない
  (`CompileError`) ときの `invalid_file`・`unsupported_version`・`unsupported_features`・`invalid_model`・`compile`・`read_limit_exceeded`。
  wasm の `LoadError` と C ABI の `ugoe_puppet_create_from_*` が同じ値を返すためにここで決める (T0457)。補足の値は `LoadErrorValue`
  (数か文。上限のバイト数が `usize` に収まらないことがあるので `FieldValue` とは別)
- [docs/spec/file-formats.md#読み込みの上限](../../docs/spec/file-formats.md#読み込みの上限) (`limits`: 上限は `ugoe::format::read_bundle_with` /
  `ugoe::format::module_file::read_module_with` に `ReadLimits` を渡して指定する。超えたときの種類と補足の値 `ReadLimitExceeded` は
  wasm と C ABI が同じ値を返すためにここで決める。T0152)
- [docs/portability.md#ホストから来る数](../../docs/portability.md#ホストから来る数) (NaN・無限大・負の `dt` の扱い。前提はこのクレートと
  `ugoe-runtime` で決め、C ABI・wasm・Godot・Unity は表し方だけを持つ。`tests/host_numbers.rs`)
- [docs/spec/rendering.md#ホストでのライティング](../../docs/spec/rendering.md#ホストでのライティング) (`lighting`: `uses_lighting` / `Puppet::uses_lighting`・`bundle_normal_maps`)

## 着せ替えの失敗の補足の値

`DressUpError::fields` は `(キー, FieldValue)` の列を返す。値は ID・名前なら `FieldValue::Text`、番号・数なら `FieldValue::Index`
(読み込みの警告 `LoadWarning::fields` と同じ型)。wasm の `DressUpError` は文字列と数、C ABI の `ugoe_last_error_field` は
10 進の文字列にする。キーは種類ごとに決まり、無い値はキーごと出さない (T0412・T0157)。

| `code` | キー |
| --- | --- |
| `not_attached`・`invalid_module_name`・`duplicate_module` | `module` |
| `unknown_slot`・`missing_slot` | `module`・`slot` |
| `missing_parameter` | `module`・`parameter` |
| `parameter_not_exposed` | `module`・`slot`・`parameter` |
| `slot_occupied` | `slot`・`module` (付けようとしたもの)・`attached` (付いていたもの) |
| `unknown_mask` | `module`・`mesh`・`mask` (マスクの参照先) |
| `texture_out_of_range` | `module`・`mesh`・`texture` (番号。モジュールの `Module::textures` の添字) |
| `unknown_collider` | `module`・`rig`・`collider` |
| `pose_not_exposed` | `module`・`slot`・`part` (ポーズの要素) |
| `avatar_level` | `module`・`required` (モジュールが求めるレベル)・`actual` (本体のレベル。本体がアバタープロファイルを持たなければ無い) |
| `invalid_physics`・`invalid_composition` | `errors` (検査のエラーの数。個々のエラーは文 `Display` にだけある) |
| `compile` | なし |

検査のエラー (`ValidationError`) には機械が読む種類が無いので、個々のエラーは補足の値にしない (数だけを出す)。

## 関連タスク

- T0303 (`docs/tasks/T0303-*.md`)
- T0308 (当たり判定)
- T0121 (着せ替え)
- T0210 (ライティングの補助)
- T0412 (エラーと警告の code)
- T0157 (着せ替えの失敗の補足の値の不足を埋める)
- T0133 (受け手として進める)
- T0134 (受け手の段の順を update とそろえる)
- T0135 (受け手でもモーションのパーツの不透明度のカーブを当てる)
- T0152 (読み込みの上限)
- T0156 (`render` feature)
- T0457 (モデルの読み込みの失敗の種類)
