# ugoe

**層:** L4 SDK (依存してよいクレートは `scripts/check-layers.mjs` を参照)

ランタイム SDK のファサード。`Puppet` が 1 体分の更新 (モーション → 表情 → 目パチ・呼吸 → 物理 → リップシンク → ポーズ → 評価。順序は docs/spec/motion.md#フレーム内の適用順) をまとめる。下位のクレートを再エクスポートする。

## feature

- `render` (既定で有効): 参照レンダラ (`ugoe-render`、wgpu) を `ugoe::render` として再エクスポートする。エンジン独自のレンダラで描く
  組み込み先は `default-features = false` で外すと wgpu をビルドしない ([docs/portability.md#ugoe-クレートの-feature](../../docs/portability.md#ugoe-クレートの-feature)、T0156)。
  外したビルドの検査は `just sdk-no-render-check`。光源の型 (`ugoe::lighting::{Light, Lighting, MAX_LIGHTS}`。`ugoe-core` の再エクスポート) は
  `render` が無くても使える (T0166)。`std` を含む
- `std` (既定で有効): 下位のクレートと依存の `std` を有効にする。クレートは `no_std` + `alloc` で書くので、外しても公開 API は変わらない。
  `default-features = false` (`render` も `std` も無し) で、`Puppet` の読み込み・評価・物理・モーション・トラッキング・着せ替えを
  `std` の無い環境で使える ([docs/portability.md#ランタイム層が守る制約-移植性のため](../../docs/portability.md#ランタイム層が守る制約-移植性のため)、
  ADR 0031、T0183)。`aarch64-unknown-none` でのビルドは `just nostd-check ugoe`

## 守ること

- アプリやエンジンの組み込みで最初に使うクレートなので、API を小さく分かりやすく保つ
- `std::` を使わず `core::`・`alloc::` から使う。数学関数は `ugoe_core::math` から呼ぶ (`clippy.toml` の `disallowed_methods`)
- `ugoe-render` は `render` feature の下だけで使う。`render` の無いビルドでも通るように、`ugoe::render` の型を
  `render` の外の公開 API に出さない (doc コメントのリンクも `ugoe_render` へ張らない)
- 外から来るデータ (ファイル・ホストの引数) のどんな値でもパニックしない (lint と `tests/broken_input.rs`。
  [docs/portability.md](../../docs/portability.md#パニックしない))

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
  (数か文。上限のバイト数が `usize` に収まらないことがあるので `FieldValue` とは別)。`invalid_model` は `errors`・`error_codes`
  (着せ替えの `invalid_physics` と同じ値。`error_codes` は文。T0469)
- [docs/spec/file-formats.md#読み込みの上限](../../docs/spec/file-formats.md#読み込みの上限) (`limits`: 上限は `ugoe::format::read_bundle_with` /
  `ugoe::format::module_file::read_module_with` に `ReadLimits` を渡して指定する。超えたときの種類と補足の値 `ReadLimitExceeded` は
  wasm と C ABI が同じ値を返すためにここで決める。T0152)
- [docs/portability.md#ホストから来る数](../../docs/portability.md#ホストから来る数) (NaN・無限大・負の `dt` の扱い。前提はこのクレートと
  `ugoe-runtime` で決め、C ABI・wasm・Godot・Unity は表し方だけを持つ。`tests/host_numbers.rs`)
- [docs/spec/rendering.md#ホストでのライティング](../../docs/spec/rendering.md#ホストでのライティング) (`lighting`: 光源の型 `Light`・`Lighting`・`MAX_LIGHTS`、`uses_lighting` / `Puppet::uses_lighting`・`bundle_normal_maps`)

## 性能の計測

`examples/puppet_timing.rs` が `Puppet::update` と `Puppet::follow` の 1 フレームの時間を測る
(`cargo run --release -p ugoe --example puppet_timing`。目標と結果は [docs/testing.md#1-フレームの評価](../../docs/testing.md#1-フレームの評価))。

## 着せ替えの失敗の補足の値

`DressUpError::fields` は `(キー, FieldValue)` の列を返す。値は ID・名前なら `FieldValue::Text`、番号・数なら `FieldValue::Index`、
種類 (`code`) の一覧なら `FieldValue::Codes` (読み込みの警告 `LoadWarning::fields` と同じ型)。wasm の `DressUpError` は文字列と数
(一覧は `, ` でつないだ文字列)、C ABI の `ugoe_error_field` は文字列 (番号は 10 進、一覧は `, ` でつないだもの) にする。キーは種類ごとに決まり、無い値はキーごと出さない (T0412・T0157)。

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
| `invalid_physics`・`invalid_composition` | `errors` (検査のエラーの数)・`error_codes` (検査のエラーの種類の一覧) |
| `compile` | なし |

`error_codes` は検査のエラーの種類 (`ValidationError::code`。`unknown_reference`・`duplicate_id` など) を出た順に、
重なりを除いて並べたもの。外に出すときは `, ` でつなぐ (例: `"unknown_reference, duplicate_id"`。読み込みの失敗の `features` と同じつなぎ方)。
個々のエラーの中身 (どの ID か) は文 (`Display`) にだけある (T0168)。

検査の失敗の `errors`・`error_codes` は `ugoe::validation_fields` が作る。読み込みの `invalid_model` (`LoadError::fields`) と、
組み込み先 (wasm・C ABI) の設定の差し替え (物理の `invalid_physics`・エフェクトの `invalid_model`) も同じ値を使う (T0469)。

読み込みの警告 `physics_removed` (wasm の `load_json` で読み直したとき、引き継いだ物理の設定を外した) も、外した理由が
検査のエラーなら `reason` に加えて同じ `errors`・`error_codes` を持つ (`LoadWarning::PhysicsRemoved` の `codes`。T0168)。

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
- T0183 (`no_std` + `alloc`、feature `std`)
