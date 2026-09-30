# ugoe-ffi

**層:** L5 バインディング (依存してよいクレートは `scripts/check-layers.mjs` を参照)

C ABI。Unity・Unreal・Godot などから使う。規約は [docs/portability.md](../../docs/portability.md#c-abi-ugoe-ffi-の規約)。

## 守ること

- `unsafe` はこのクレートでだけ許可する
- パニックを FFI の境界の外へ漏らさない (公開する関数はすべて `error::guard` を通す)
- ABI を壊したら `ABI_VERSION` を上げる ([ABI の版](#abi-の版))
- 公開する関数・型を変えたら `just ffi-header` でヘッダを作り直してコミットする

## 使い方

ヘッダは [`include/ugoe.h`](include/ugoe.h)、ライブラリは `cargo build -p ugoe-ffi --release` で
`target/release/` にできる `libugoe_ffi.{a,so,dylib}` (Windows では `ugoe_ffi.{lib,dll}`)。
静的ライブラリをリンクするときは、Linux ではさらに `-lm -lpthread -ldl` が要る。

```c
UgoePuppet *puppet = NULL;
if (ugoe_puppet_create_from_bundle(bytes, len, &puppet) != UGOE_OK) {
    fprintf(stderr, "%s\n", ugoe_last_error());
}
ugoe_puppet_set_parameter(puppet, index, value);
ugoe_puppet_update(puppet, dt);
const float *xy; size_t n;
ugoe_puppet_positions(puppet, drawable, &xy, &n);  /* 次の update まで有効 */
ugoe_puppet_destroy(puppet);
```

動く見本は [`examples/c/sample.c`](examples/c/sample.c) (`just ffi-check` でビルドし、`assets/samples/avatar.ugoe`・
法線マップ付きの `avatar-lit.ugoe`・着せ替えの `dress-up/` (本体とモジュール) で実行する)。

- 関数は `UgoeResult` (`UGOE_OK` = 0) を返し、失敗したら同じスレッドの `ugoe_last_error()` でメッセージを得る。
- 内部でパニックが起きたら `UGOE_ERROR_PANIC` を返す。そのオブジェクトの状態は不定なので破棄する。
- 頂点位置はホストのバッファへのコピー (`ugoe_puppet_copy_positions`) と、内部のバッファの借用
  (`ugoe_puppet_positions`。次の `ugoe_puppet_update` か破棄まで有効) の両方がある。
- テクスチャはエンコードされたままのバイト列 (PNG・KTX2) を渡す。デコードと乗算済みアルファへの変換はホストが行う
  ([docs/spec/rendering.md](../../docs/spec/rendering.md))。
- 1 つの `UgoePuppet` を複数のスレッドから同時に使ってはならない。

## ヘッダの生成

ヘッダは [cbindgen](https://github.com/mozilla/cbindgen) の CLI で生成する (設定は `cbindgen.toml`)。
cbindgen は MPL-2.0 なので、このクレートの依存 (`build-dependencies`・`dev-dependencies` を含む) には入れず、
ワークスペースの外のツールとして使う。版は `scripts/ffi.mjs` の `CBINDGEN_VERSION` と CI (`.github/workflows/ci.yml` の `cbindgen@...`) で揃え、
違う版や未導入なら `just ffi-check` / `just ffi-header` が入れ方を案内して止まる。ヘッダには cbindgen の版を書かない。

`just ffi-check` は cbindgen と C コンパイラ (`cc`。`CC` で変えられる) が要るので `just ci` には含めず、CI の `rust` ジョブで別に実行する。

```sh
cargo install cbindgen --version 0.29.4 --locked
just ffi-header   # include/ugoe.h を作り直す
just ffi-check    # ヘッダが最新かの検査と、C の見本のビルド・実行
```

## ABI の版

`ugoe_abi_version()` (ヘッダでは `UGOE_ABI_VERSION`) は、**既存の** 関数・型・定数を壊したときだけ上げる (T0409 で決めた)。

- 上げる: 既存の関数の引数・戻り値の型や意味、構造体のフィールドの並び・大きさ、定数の値を変えたとき、関数や型を消したとき。
- 上げない: 関数・型・定数・エラーコードを **足す** だけのとき。古いヘッダで作ったホストは、足したものを使わないので
  そのまま動く。新しい関数があるかは、ライブラリの版 (`ugoe_version()`) で確かめる。
- そのため、構造体にフィールドを足したくなったら、既存の構造体は変えずに別の関数や構造体を足す
  (例: 描画可能要素がライティングを受けるかは `UgoeDrawableInfo` に足さず、`ugoe_puppet_drawable_lit` にした)。
- 知らないエラーコードが返ることがある (足したもの)。0 以外はすべて失敗として扱い、`ugoe_last_error()` を出せばよい。

## 着せ替え (T0409)

```c
UgoeModule *hair = NULL;
ugoe_module_create(bytes, len, &hair);            /* .ugoemod を読む (付けはしない)。読めなければ UGOE_ERROR_FORMAT */
if (ugoe_puppet_attach_module(puppet, hair) != UGOE_OK) {
    /* UGOE_ERROR_DRESS_UP: 種類と補足の値は wasm の DressUpError と同じ */
    printf("%s (%s)\n", ugoe_last_error_kind(), ugoe_last_error_field("slot"));
}
const uint32_t *changed; size_t n;
ugoe_puppet_changed_textures(puppet, &changed, &n); /* 画像を設定し直すテクスチャの番号 */
ugoe_puppet_detach_module(puppet, "LongHair");
ugoe_puppet_set_modules(puppet, list, count);       /* 付ける組をまとめて決める (合成の順)。0 個で本体だけ */
ugoe_module_destroy(hair);                          /* 付けたモジュールは付いたまま残る */
```

- 規則は `docs/spec/modules.md#puppet-での着せ替え` (`ugoe::Puppet::attach_module` などと同じ)。同じスロットか同じ名前の
  モジュールは差し替える。失敗したときは何も変えない。
- **付け外しに成功すると、モデルが差し替わる。** 描画可能要素の数・並び・ID が変わり、この `UgoePuppet` から借りたポインタ
  (パラメータ・描画可能要素・当たり判定の ID、頂点・UV・インデックス・描画順、付けているモジュールの名前、外したモジュールのテクスチャ)
  は無効になる。描画可能要素の情報は取り直す。
- テクスチャは本体、モジュールの順に並ぶ (`ugoe::Puppet::texture_sources`)。付け外しの後は `ugoe_puppet_changed_textures` の番号の
  画像と法線マップを `ugoe_puppet_texture` / `ugoe_puppet_normal_map` で引き直し、`ugoe_puppet_texture_count` 以上の番号は捨ててよい。
- `UgoeModule` は `Arc` で共有するので、破棄しても付けたモジュール (と画像) は残り、複数の `UgoePuppet` に付けてよい。
- スロットは `ugoe_puppet_slot_count` / `ugoe_puppet_slot_id` (本体のもの)、付けているモジュールは
  `ugoe_puppet_attached_module_count` / `ugoe_puppet_attached_module_name` (合成の順)。

失敗の種類 (`ugoe_last_error_kind()`) は wasm の `DressUpError` の `code` と同じで、同じ補足の値
(`ugoe_last_error_field("module")`・`"slot"`・`"parameter"`・`"mesh"`・`"attached"`) を持つ
([crates/ugoe-wasm/README.md](../ugoe-wasm/README.md#着せ替え-t0123))。

| 種類 | 戻り値 | 補足の値 |
| --- | --- | --- |
| `invalid_file` | `UGOE_ERROR_FORMAT` (`ugoe_module_create`) | |
| `unknown_slot`・`missing_slot` | `UGOE_ERROR_DRESS_UP` | `module`・`slot` |
| `missing_parameter` | 同上 | `module`・`parameter` |
| `parameter_not_exposed` | 同上 | `module`・`slot`・`parameter` |
| `slot_occupied` | 同上 | `slot`・`module`・`attached` |
| `unknown_mask`・`texture_out_of_range` | 同上 | `module`・`mesh` |
| `pose_not_exposed` | 同上 | `module`・`slot` |
| `avatar_level`・`unknown_collider`・`invalid_module_name`・`duplicate_module`・`not_attached` | 同上 | `module` |
| `invalid_composition`・`compile`・`invalid_physics` | 同上 | |

wasm にある `not_loaded`・`no_model`・`texture` は C ABI では起きない (モジュールはポインタで渡し、`UgoePuppet` は常にモデルを持ち、
画像の登録はホストが行う)。種類を持たない失敗では `ugoe_last_error_kind()` は NULL。対応は `src/dress_up.rs` と
`ugoe-wasm` の `dress_up::error_info` の 2 か所にあるので、種類を足すときは両方を直す。

## ライティング (T0409)

- `ugoe_puppet_uses_lighting` (光源の操作を出すか)、`ugoe_puppet_drawable_lit` (その要素に陰影を掛けるか)、
  `ugoe_puppet_normal_map` (テクスチャの番号ごとの法線マップのバイト列。無ければ `UGOE_TEXTURE_NONE` で、法線 `(0, 0, 1)` として扱う)。
  法線マップの PNG は **アルファを掛けずに** デコードする。
- 光源はホストの状態 (`UgoePuppet` は持たない)。`ugoe_lighting_uniform(ambient, lights, count, &uniform)` が光源の組
  (`UgoeLight`。`ugoe::render::Lighting` と同じ中身) を検査し、参照シェーダのユニフォームと同じ配置 (`UgoeLightingUniform`。
  平行光源は光源へ向かう単位ベクトル、点光源は位置と `1 / range`) にする。エンジンのシェーダで陰影の式
  (`docs/spec/rendering.md#ライティング`) を書くときに、そのまま定数バッファに入れられる。
- 向きが 0・有限でない値・`range` が正でない・負の色は `UGOE_ERROR_INVALID_ARGUMENT` (wasm の `set_lighting_json` と同じく先に知らせる)。
  `UGOE_MAX_LIGHTS` (8) を超える光源は先頭だけを使う。

## 当たり判定・トラッキング・そのほか (T0409)

- 当たり判定: `ugoe_puppet_hit_area_count` / `_id`、`ugoe_puppet_hit_areas_at` (手前から順の番号)、`ugoe_puppet_hit_test` (ID)、
  標準の名前 (`"Head"`・`"Body"`) の `ugoe_puppet_standard_hit_area` / `ugoe_puppet_hit_test_standard`。直前の `update` の形で判定する。
- 可変長の結果 (`ugoe_puppet_hit_areas_at`・`ugoe_puppet_tracking_targets`) は、ホストのバッファ (`out`・`capacity`) に書き、数を
  `*out_count` に書く。足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で数だけを書くので、`capacity` 0 で数を聞いてから呼び直せる。
- トラッキング: `ugoe_puppet_set_tracking(puppet, &input)` (NULL で入力をやめる)。`UgoeTrackingInput` は頭の回転、
  体の回転と肩 (NaN で省略)、ARKit の 52 のブレンドシェイプ (添字は `ugoe_blend_shape_name` / `ugoe_blend_shape_index`。
  MediaPipe のカテゴリ名をそのまま引ける)。平滑化などはホストが行う。トラッキング中は `ugoe_puppet_set_stage_enabled` で
  目パチ (`UGOE_STAGE_EYE_BLINK`) を止めるとよい。書き込むパラメータは `ugoe_puppet_tracking_targets`。
- `ugoe_puppet_set_parameters_by_id` (ID でまとめて設定。知らない ID は飛ばし、設定した数を返す)、
  `ugoe_puppet_stop_expressions` (表情の解除)。

## 読み込みの警告 (T0409)

`ugoe_puppet_load_warning_count` / `ugoe_puppet_load_warning` (`UgoeWarning { code, message }`) /
`ugoe_puppet_load_warning_field` (補足の値。数も 10 進の文字列)。`.ugoemod` は `ugoe_module_warning_*`。
`code` は wasm の `load_warnings_json` と同じ値を使う ([crates/ugoe-wasm/README.md](../ugoe-wasm/README.md#読み込みの警告-t0125))。

| `code` | 補足の値 | いつ |
| --- | --- | --- |
| `texture_missing` | `texture` | テクスチャの画像が無い (読み込みが数を検査するので通常は起きない) |
| `unknown_optional_feature` | `feature` (機能の ID) | 知らない任意の機能を宣言したファイル。フォールバックで読んだ (C ABI だけ。wasm はまだ出さない) |
| `unknown_value` | `path` (チャンクと場所) | 知らない列挙の値をフォールバックで読んだ (C ABI だけ) |

- wasm の `texture_ktx2`・`normal_map_ktx2` は C ABI では出さない (画像のデコードはホストが行うので、KTX2 を扱えるかはホストが決める)。
  `physics_removed`・`module_detached` はモデルの読み直しで起きるもので、C ABI には読み直しが無いので起きない。
- JSON から作ったときは警告を出さない (JSON はもともとテクスチャを持たない)。
- `message` は英語の文で、安定した値として扱わない。知らない `code` は `message` をそのまま出せばよい。

## 範囲外 (後続)

- Windows と macOS での C の見本のビルドは確かめていない (CI は Linux だけ)。
- 描画 (`ugoe-render`) は C ABI に出さない。ホストのエンジンで描く (段階 B。`docs/portability.md`)。

## 関連タスク

- T0401 (`docs/tasks/T0401-*.md`)
- T0409 (着せ替え・ライティング・当たり判定・トラッキング・読み込みの警告)
