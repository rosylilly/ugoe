---
id: T0457
title: C ABI のモデルの読み込みの失敗にも種類を付けて wasm の LoadError と揃え、wasm の着せ替えの GPU の喪失にコードを付ける
status: done
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, docs]
depends_on: []
assignee: claude-worker-T0457
---

# T0457: C ABI のモデルの読み込みの失敗にも種類を付けて wasm の LoadError と揃え、wasm の着せ替えの GPU の喪失にコードを付ける

## 目的

T0434 の残り (T0434 の候補)。

- C ABI のモデルの読み込み (`ugoe_puppet_create_from_*`) の失敗は、上限を超えたときだけ `ugoe_last_error_kind` を持つ。wasm の `LoadError` の
  コード (`invalid_file`・`unsupported_version`・`invalid_model`・`compile` など) と揃える。種類を決める処理は SDK (`ugoe` クレート) に置き、wasm と C ABI で共有する。
- wasm の着せ替えの付け外しで GPU を失ったときの例外 (`with_wardrobe!` の「the GPU device was lost」) にコードが無い。`DressUpError` の `device_lost` を付ける。

## 受け入れ条件

- [x] 読み込みの失敗の種類を SDK で決め (`LoadError` 相当の `code()`)、wasm と C ABI がそれを使う
- [x] C ABI の読み込みの失敗ごとに `ugoe_last_error_kind` の値を確かめるテストがあり、`ugoe.h` の説明を更新した (Unity の `UgoeResult`・`Fields` に影響があればメモに)
- [x] wasm の着せ替えで GPU を失ったときに `name: "DressUpError"`・`code: "device_lost"` になる (ネイティブのテストで)
- [x] crates/ugoe-wasm/README.md・crates/ugoe-ffi/README.md・`codes.json` を更新した
- [x] `just ci`・`just wasm-check`・`just conformance` が通る

## 参考

- T0434 のメモ (起票の候補)、crates/ugoe-wasm/README.md#読み込みの失敗-t0434

## メモ

- T0026 で起票した (元の候補: T0434 の C ABI の種類と wasm の `device_lost`)。
- やったこと:
  - SDK に `ugoe::LoadError` (`Format(FormatError)`・`Compile(CompileError)`) と `code()`・`fields()`・`exceeds_limit()`、補足の値の
    `LoadErrorValue` (`Number(u64)`・`Text(String)`) を足した (`crates/ugoe/src/load_error.rs`)。`FieldValue` は `usize` と借用の文なので、
    上限のバイト数 (u64) と機能の ID をつないだ文を持てず、別の型にした。
  - wasm の `LoadError` の `Format`・`Compile` を `Model(ugoe::LoadError)` 1 つにまとめ (`From<FormatError>`・`From<CompileError>` は残す)、
    `info()` は SDK の値を写すだけにした (`format_error_info` を消した)。JS に出る値は変わらない (`codes.json` の `load_error` は差分なし)。
  - C ABI の `ugoe_puppet_create_from_bundle(_with_limits)`・`ugoe_puppet_create_from_json` の失敗に、SDK の種類と補足の値 (10 進の文字列) を
    付けた (`limits::load_error`)。**戻り値は変えていない** (`UGOE_ERROR_FORMAT`・`UGOE_ERROR_INVALID_MODEL`・`UGOE_ERROR_LIMIT_EXCEEDED`)
    ので ABI の版は上げない。バンドルを読むときの検査に通らないモデルは `UGOE_ERROR_FORMAT` で種類 `invalid_model`、JSON (読むときに検査しない) は
    `UGOE_ERROR_INVALID_MODEL` で `invalid_model`。文 (`ugoe_last_error`) も前と同じ。UTF-8 でない JSON は今までどおり種類なし。
    テストは `crates/ugoe-ffi/tests/load_errors.rs` (`assets/compat/future` の新しい版・必須の機能のファイルを使う)、コンパイルの失敗は
    `limits.rs` の単体テスト (検査を通って評価できない入力は作りにくいため)。`tests/limits.rs` の「種類なし」は `invalid_file` に直した。
  - wasm の着せ替えの `with_wardrobe!` で GPU を失ったときは、`WardrobeError::DeviceLost` (`name` `"DressUpError"`、`code` `device_lost`、
    文は前と同じ) にした。`WardrobeError::NAME` を足し、ネイティブのテスト `device_lost_is_a_coded_dress_up_error` で確かめる。
    `codes.json` の `dress_up_error` に `device_lost` が増えた。
  - `ugoe.h` を作り直した (説明だけ)。README (ugoe・ugoe-ffi・ugoe-wasm) と `docs/portability.md` を直した。
  - crates: の外 (小さな変更): `bindings/web/src/warnings.ts` の `DRESS_UP_ERROR_CODES` に `device_lost` を足した (`codes.test.ts` が
    `codes.json` と比べるため)。
  - Unity: `UgoeException.FieldKeys` は `found`・`supported`・`features`・`errors` を既に含むので、コードの変更は要らない。`UgoeResult.Format`・
    `InvalidModel` と `UgoeException` の説明 (「`Kind` は着せ替えの失敗で」) は古くなったが、crates: の外なので直していない。
- やらなかったこと:
  - 実際のブラウザでの確認 (`with_wardrobe!` の例外の形は `dress_up_error` を通るので、ネイティブのテストと `just wasm-check` まで)。
  - `render`・`readback` などほかの「the GPU device was lost」(コード無しの `JsError`) は変えていない (受け入れ条件は着せ替えだけ)。
  - `@ugoe/player` の `describeDressUpError` は `device_lost` の訳を持たない (知らないコードとして英語の文を出す)。
- 起票の候補:
  - `@ugoe/player` の着せ替えの失敗の訳に `device_lost` を足す (`loadError.deviceLost` と同じ文でよい)。
  - Unity の `UgoeResult.Format`・`InvalidModel`・`UgoeException` の説明を、モデルの読み込みも種類を持つことに合わせて直し、
    `Puppet.FromBundle` の失敗の `Kind` を確かめるテストを足す。
  - wasm の `render`・読み戻しなど、ほかの GPU の喪失の例外にもコードを付けるか決める。
