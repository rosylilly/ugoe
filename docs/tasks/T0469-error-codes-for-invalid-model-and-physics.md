---
id: T0469
title: "読み込みの invalid_model に error_codes、物理の差し替えの失敗に invalid_physics の種類、@ugoe/runtime に set_effects_json の例外を書く"
status: in-progress
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

- [ ] 読み込みの `invalid_model` の補足の値に `error_codes` がある (`errors` は残す)。SDK・C ABI・wasm のテストがある
- [ ] C ABI と wasm の物理の設定の差し替え (`set_physics_json`・`set_base_physics_json`) の検査の失敗が、種類 `invalid_physics` と `errors`・`error_codes` を持つ。テストがある
- [ ] `@ugoe/runtime` の型 (`codes.json` から作る一覧を含む) と README に、`set_effects_json`・`set_physics_json`・`set_base_physics_json` の例外の種類と補足の値が書いてある
- [ ] crates/ugoe-ffi/README.md・crates/ugoe-wasm/README.md・bindings/unity の doc コメントと README を直した (Unity は T0464 の列挙で読むので、コードの変更は要らない見込み)
- [ ] `just ci`・`just wasm-check`・`just refs` が通る

## 参考

- T0168・T0461 のメモ (起票の候補)
- crates/ugoe/src/load_error.rs、crates/ugoe/src/dress_up.rs (`validation_fields`)、crates/ugoe-wasm/src/animation.rs

## メモ

- T0036 で起票した (元の候補: T0168 (2 つ)・T0461 (`@ugoe/runtime`))。
