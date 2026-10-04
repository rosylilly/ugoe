---
id: T0157
title: 着せ替えの失敗の補足の値の不足を埋める
status: done
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, docs]
depends_on: [T0139]
assignee: claude-worker-T0157
---

# T0157: 着せ替えの失敗の補足の値の不足を埋める

## 目的

`DressUpError::fields` (T0412。wasm の `DressUpError` と C ABI の `ugoe_last_error_*` が使う) は、`ComposeError` が持つ値の一部を落としている。
組み込み先が利用者に分かる文を作るのに要る値が無い (T0139 のレビューで見つけた)。

| `code` | 落ちている値 |
| --- | --- |
| `unknown_mask` | `mask` (参照先) |
| `unknown_collider` | `rig`・`collider` |
| `pose_not_exposed` | `part` |
| `texture_out_of_range` | `texture` (番号) |
| `avatar_level` | `required`・`actual` (レベル) |
| `invalid_physics`・`invalid_composition` | 検査のエラー (どれか 1 つでも) |

キーを足すかを決め、足すなら `fields` (値が番号のものは `FieldValue` のような型にするか) と wasm・C ABI の出し方をそろえる。
`code` の値は変えない。

## 受け入れ条件

- [x] 足すキーと値の型を決め、crates/ugoe/README.md と `DressUpError::fields` の doc に書いた
- [x] wasm (`crates/ugoe-wasm/src/dress_up.rs`) と C ABI (`crates/ugoe-ffi/src/dress_up.rs`・`error.rs`) が同じキーを出すことをテストで確かめた
- [x] `just ci` が通る

## 参考

- crates/ugoe/src/dress_up.rs (`DressUpError::info`)
- crates/ugoe-runtime/src/compose.rs (`ComposeError`)
- docs/spec/modules.md#puppet-での着せ替え

## メモ

### やったこと

- **決めたこと:** 落ちていた値はすべて足した。`DressUpError::fields` の戻り値を `Vec<(&str, &str)>` から `Vec<(&str, FieldValue)>` に変えた
  (読み込みの警告 `LoadWarning::fields` と同じ型。ID・名前は `Text`、番号・数は `Index`)。`code` の値は変えていない。
  - `unknown_mask`: `mask`。`unknown_collider`: `rig`・`collider`。`pose_not_exposed`: `part`。
  - `texture_out_of_range`: `texture` (番号。モジュールの `Module::textures` の添字)。
  - `avatar_level`: `required`・`actual` (番号)。本体がアバタープロファイルを持たない (`actual: None`) ときは `actual` のキーを出さない。
  - `invalid_physics`・`invalid_composition`: `errors` (検査のエラーの数)。`ValidationError` には機械が読む種類が無く、個々のエラーの
    英語の文は `Display` (wasm の `message`・`ugoe_last_error`) に既にあるので、補足の値には数だけを出す (owned の文を返すために
    `FieldValue` を `Copy` でなくする変更はしなかった)。
- 出し方: wasm は番号を JSON の数 (`load::field_json` を `warning_json` と共有)、C ABI は 10 進の文字列。無い値は wasm ではキーが無く、C ABI では NULL。
- 書いた所: `DressUpError::fields` の doc、crates/ugoe/README.md (#着せ替えの失敗の補足の値 の表)、crates/ugoe-ffi/README.md の表と
  `ugoe_last_error_field` の doc (ヘッダを `just ffi-header` で作り直した)、crates/ugoe-wasm/README.md、docs/spec/modules.md#puppet-での着せ替え。
- テスト: `ugoe` の全種類の表のテストを新しい値に直した。wasm (`numbered_and_referenced_fields_reach_js`) と C ABI
  (`numbered_and_referenced_fields_reach_the_c_abi`) に同じ表 (新しいキーを持つ 8 件) のテストを置き、同じキーが出ることを確かめた。
  実際の付け外しでも `avatar_level` の `required` が出て `actual` が無いことを、wasm (`failed_attach_reports_numbered_fields`) と
  C ABI (`tests/extended.rs` の `dress_up_failures_carry_the_wasm_codes`) で確かめた。
- `just ci` と `just wasm-check` は通った (2026-10-04、main の 812ff0f の上)。

### やらなかったこと

- `ugoe_puppet_set_base_physics` (C ABI) の検証の失敗は従来どおり `UGOE_ERROR_INVALID_MODEL` で種類を持たない (README の表に書いた)。
- 範囲外の bindings は触っていない (下の起票の候補)。

### 起票の候補

- **bindings/web:** `bindings/web/src/warnings.ts` の `UgoeDressUpErrorFields` に `mask?`・`rig?`・`collider?`・`part?` (string)、
  `texture?`・`required?`・`actual?`・`errors?` (number) を足す。README の `e.module・...` の一覧も。
- **bindings/unity:** `Runtime/Core/UgoeException.cs` の `FieldKeys` が固定の一覧 (`module`・`slot`・`parameter`・`mesh`・`attached`・
  `limit`・`chunk`・`max`) なので、新しいキー (`mask`・`rig`・`collider`・`part`・`texture`・`required`・`actual`・`errors`) が `Fields` に入らない。
  一覧に足すか、C ABI にキーの列挙を足す。
- 検査のエラー (`ValidationError`) に機械が読む種類を持たせる (補足の値に個々のエラーを出せるように)。要る組み込み先が出たら。

