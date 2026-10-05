---
id: T0469
title: "読み込みの invalid_model に error_codes、物理の差し替えの失敗に invalid_physics の種類、@ugoe/runtime に set_effects_json の例外を書く"
status: done
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, bindings/web, bindings/unity, docs]
depends_on: []
assignee: claude-worker-T0469
---

# T0469: 読み込みの invalid_model に error_codes、物理の差し替えの失敗に invalid_physics の種類、@ugoe/runtime に set_effects_json の例外を書く

## 目的

T0168・T0461 で失敗の種類と補足の値を揃えたが、次が残っている。どれも「キーや種類を足すだけ」で、既存の値の意味は変えない。

1. 読み込みの失敗 `invalid_model` (`ugoe::LoadError::fields`、`crates/ugoe/src/load_error.rs`) は `errors` (数) だけで、
   T0168 で着せ替え・`physics_removed` に足した `error_codes` (検査のエラーの種類を `, ` でつないだもの) が無い (T0168 の候補)。
2. 物理の設定の差し替えの検査の失敗は種類を持たない: C ABI の `ugoe_puppet_set_physics_json`・`ugoe_puppet_set_base_physics_json` は
   `UGOE_ERROR_INVALID_MODEL` と文だけ (`crates/ugoe-ffi/src/animation.rs` の `invalid_physics`)、wasm の `set_physics_json`・
   `set_base_physics_json` も `AnimationError::InvalidPhysics` の `info` が `None` (T0168 の候補。候補は `set_base_physics` だけを挙げたが、
   同じ関数を使う `set_physics_json` も同じ)。着せ替えの `DressUpError::Physics` と同じ `invalid_physics`・`errors`・`error_codes` を付ける。
3. `@ugoe/runtime` (bindings/web) の型と README に、`set_effects_json` の例外 (`code: "invalid_model"`・`errors`) が書かれていない (T0461 の候補)。
   2. の物理の差し替えの例外もあわせて書く。

## 受け入れ条件

- [x] 読み込みの `invalid_model` の補足の値に `error_codes` がある (`errors` は残す)。SDK・C ABI・wasm のテストがある
- [x] C ABI と wasm の物理の設定の差し替え (`set_physics_json`・`set_base_physics_json`) の検査の失敗が、種類 `invalid_physics` と `errors`・`error_codes` を持つ。テストがある
- [x] `@ugoe/runtime` の型 (`codes.json` から作る一覧を含む) と README に、`set_effects_json`・`set_physics_json`・`set_base_physics_json` の例外の種類と補足の値が書いてある
- [x] crates/ugoe-ffi/README.md・crates/ugoe-wasm/README.md・bindings/unity の doc コメントと README を直した (Unity は T0464 の列挙で読むので、コードの変更は要らない見込み)
- [x] `just ci`・`just wasm-check`・`just refs` が通る

## 参考

- T0168・T0461 のメモ (起票の候補)
- crates/ugoe/src/load_error.rs、crates/ugoe/src/dress_up.rs (`validation_fields`)、crates/ugoe-wasm/src/animation.rs

## メモ

- T0036 で起票した (元の候補: T0168 (2 つ)・T0461 (`@ugoe/runtime`))。
- やったこと (T0469):
  - SDK: 検査の失敗の補足の値 (`errors`・`error_codes`) を作る `ugoe::validation_fields` を公開し (着せ替えの `validation_fields` を
    `warning.rs` へ移した)、`LoadError::fields` の `invalid_model` に `error_codes` (`LoadErrorValue::Text`。`, ` でつないだもの) を足した。
    `LoadErrorValue` に列挙子は足していない (網羅の `match` を壊さないため)。
  - C ABI: `ugoe_puppet_set_physics_json`・`ugoe_puppet_set_base_physics_json` の検査の失敗に種類 `invalid_physics` と `errors`・`error_codes`
    を付けた (戻り値は `UGOE_ERROR_INVALID_MODEL` のまま。種類を足しただけなので ABI の版は上げない)。読み込みの `invalid_model` は SDK から
    `error_codes` が入る。`ugoe_puppet_set_effects_json` の `invalid_model` にも `error_codes` を足した (「読み込みの `invalid_model` と同じ」を保つため)。
    テストは `tests/load_errors.rs`・`tests/extended.rs`・`tests/host.rs`。ヘッダを作り直した。
  - wasm: `AnimationError::info` が `InvalidPhysics` に `invalid_physics`、`InvalidEffects` に `invalid_model` を返し、どちらも `errors`・`error_codes`
    を持つ。`set_physics_json`・`set_base_physics_json` は `set_effects_json` と同じく `name` が `"Error"` の coded error を投げる
    (戻り値の型を `JsError` から `JsValue` にした)。`codes.json` に `settings_error` の組を足し (`src/codes.rs` の `AnimationError` の網羅の見本)、
    `load_error.invalid_model` に `error_codes` が入った。
  - `@ugoe/runtime`: `src/settings-errors.ts` (`SETTINGS_ERROR_CODES`・`UgoeSettingsError*`・`isSettingsError`) を足し、`UgoeLoadErrorFields` に
    `error_codes`、`codes.test.ts` に `settings_error` の突き合わせ、README に「設定の差し替えの失敗」の節を足した。`isSettingsError` は
    `name` で見分けられないので、知っている `code` だけを受ける。
  - Unity: コードは変えず (T0464 の列挙で入る)、doc コメントと README を直し、`tests/ResultTests.cs` に実際の失敗の種類とキーのテストを足した。
  - docs/portability.md のホストから来る値の表を直した。
  - `just ci`・`just wasm-check`・`just refs`・`just ffi-check`・`just unity-check`・`just conformance` が通った (rebase 後)。
- やらなかったこと: `set_base_physics_json` で合成に失敗したとき (wasm の `AnimationError::DressUp`) の例外は `code` を持たないまま
  (C ABI は `UGOE_ERROR_DRESS_UP` と種類を持つ)。例外の `name` (`"DressUpError"` にするか) の判断が要るので触らなかった。
- 起票の候補:
  - wasm の `set_base_physics_json` の合成の失敗 (`AnimationError::DressUp`) に、着せ替えの種類と補足の値を付ける (C ABI と揃える。`name` を決める)。
  - bindings/godot の読み込みの `invalid_model` (`bindings/godot/src/model.rs` の `info`) にも `error_codes` を足す (このタスクの範囲の外。
    SDK の `ugoe::LoadError::fields` を使うように揃えるとよい)。godot の物理の差し替えの失敗の文 (`node/sdk.rs` の `invalid_physics`) も同様。

