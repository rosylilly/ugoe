---
id: T0409
title: C ABI: 着せ替え・ライティング・当たり判定・トラッキング・読み込みの警告を足す
status: done
milestone: M7
crates: [ugoe-ffi, docs, scripts]
depends_on: [T0401]
assignee: claude-worker-T0409
---

# T0409: C ABI の拡張

## 目的

T0401 の後続の候補。C ABI は読み込み・パラメータ・update・描画可能要素・テクスチャ・モーションと表情までで、
Unity などのホストが使いたい機能 (着せ替え・ライティング・当たり判定・トラッキング・読み込みの警告) がまだない。

## 受け入れ条件

- [x] 着せ替え: `.ugoemod` の読み込み、付け外し、スロットの一覧、失敗の構造化されたエラー (wasm の `DressUpError` と同じ code)
- [x] ライティング: 法線マップのバイト列の借用、光源の設定 (ランタイムの `Lighting` に合わせる)
- [x] 当たり判定 (`hit_test` と標準の当たり判定)、トラッキングの入力、読み込みの警告 (wasm の `load_warnings_json` と同じ code)、ID によるパラメータの一括設定、表情の解除
- [x] ABI を壊す変更なら `ugoe_abi_version()` を上げる。ヘッダを作り直し、C の見本を広げる
- [x] `just ci`・`just wasm-check`・`just ffi-check` が通る。Windows と macOS での C の見本のビルドは、できなければ利用者の確認に残す

## 参考

- `docs/tasks/T0401-ffi.md` のメモ、`crates/ugoe-ffi/README.md`、`docs/portability.md`

## メモ

- 範囲外の変更: `scripts/ffi.mjs` (`just ffi-check` で C の見本を `avatar.ugoe` に加えて `avatar-lit.ugoe` (`--lit`) と
  `dress-up/` (本体と 4 つの `.ugoemod`) でも実行する)。team-lead 承認済み。理由: C の見本の実行は scripts/ffi.mjs にあり (T0401 と同じ)、着せ替え・ライティングの API を実際の見本で確かめるため。justfile と CI は変えていない。
- ABI の版は 1 のまま。**足すだけなら上げない** と決め、`crates/ugoe-ffi/README.md#abi-の版` と `docs/portability.md` に書いた。
  既存の構造体は変えない (`UgoeDrawableInfo` に `lit` を足さず `ugoe_puppet_drawable_lit` にした)。
- 着せ替え: `.ugoemod` は不透明な `UgoeModule` (`ugoe_module_create` / `_destroy`・名前・スロット・警告) で、中身を `Arc` で共有する
  (破棄しても付けたモジュールは残る。複数の `UgoePuppet` に付けられる)。wasm のように Puppet に名前で読み込む形にはしなかった
  (C では寿命をオブジェクトで持つ方が素直なため)。付け外しは `ugoe_puppet_attach_module`・`_detach_module` (名前)・`_set_modules`、
  一覧は `ugoe_puppet_attached_module_*`・`ugoe_puppet_slot_*`、設定し直すテクスチャは `ugoe_puppet_changed_textures`。
  テクスチャ・法線マップは `Puppet::texture_sources` で本体とモジュールの画像を引く。付け外しに成功すると、借りたポインタ
  (ID・頂点・UV・インデックス・描画順) は無効になる (各関数の doc と README に書いた)。
- 構造化されたエラー: 新しいコード `UGOE_ERROR_DRESS_UP` (9) と、スレッドローカルの `ugoe_last_error_kind()`・`ugoe_last_error_field(key)`。
  種類と補足の値は wasm の `DressUpError` と同じ (`invalid_file` は `UGOE_ERROR_FORMAT`)。`not_loaded`・`no_model`・`texture` は C ABI では起きない。
  対応表は `ugoe-ffi/src/dress_up.rs` と `ugoe-wasm/src/dress_up.rs` の 2 か所にある (どちらも L5 で互いに依存できないため)。
- ライティング: `ugoe_puppet_uses_lighting`・`ugoe_puppet_drawable_lit`・`ugoe_puppet_normal_map` (バイト列の借用)。光源はホストの状態なので
  `UgoePuppet` に持たせず、`ugoe_lighting_uniform` で `UgoeLight` の組を検査して参照シェーダのユニフォームと同じ配置 (`UgoeLightingUniform`) にする。
- 当たり判定 (`hit_area_*`・`hit_areas_at`・`hit_test`・`standard_hit_area`・`hit_test_standard`)、トラッキング (`UgoeTrackingInput`。体と肩は NaN で省略、
  `ugoe_blend_shape_name` / `_index`、`ugoe_puppet_tracking_targets`)、段の有効無効 (`ugoe_puppet_set_stage_enabled`。トラッキング中に目パチを止めるため。
  受け入れ条件には無いが足した)、`ugoe_puppet_set_parameters_by_id`、`ugoe_puppet_stop_expressions`。
- 読み込みの警告: `ugoe_puppet_load_warning_*` と `ugoe_module_warning_*` (`UgoeWarning { code, message }` と、キーで引く補足の値)。
  wasm と同じ `texture_missing` に加えて、`read_bundle_with_report` の `unknown_optional_feature` (`feature`)・`unknown_value` (`path`) を出す
  (wasm はまだ出さない)。KTX2 の警告はデコードがホストの仕事なので出さない。
- テスト: `crates/ugoe-ffi/tests/extended.rs` (Rust の Puppet と同じモデル・テクスチャの変化、wasm と同じエラーの種類、`assets/compat/future/` の警告など)。
  C の見本 (`examples/c/sample.c`) を広げ、3 つの見本で動かした。ヘッダは C++ (`c++ -std=c++17`) でもコンパイルできる。
- 確認: `just ci`・`just wasm-check`・`just ffi-check`・`cargo clippy -p ugoe-ffi --target wasm32-unknown-unknown` が通る (main e2c0a0d に rebase 後)。
- 確かめていないこと: Windows と macOS での C の見本のビルド (利用者の確認に残す)。
- 後続の候補:
  - wasm の `load_warnings_json` にも `unknown_optional_feature`・`unknown_value` を足す (C ABI と揃える)。
  - 着せ替えのエラーの種類の対応を `ugoe` (SDK) の `DressUpError::code()` などに移し、wasm と C ABI の重複をなくす。
  - C ABI からモーションのイベント・物理の設定の差し替え・パーツの不透明度を読む API。
