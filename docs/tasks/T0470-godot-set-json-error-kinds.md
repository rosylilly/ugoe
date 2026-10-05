---
id: T0470
title: "Godot: set_*_json の失敗の種類を返す口を持つか (判断が要る)"
status: done
milestone: M9
crates: [bindings/godot, docs]
depends_on: [T0469]
assignee: claude-worker-T0470
---

# T0470: Godot: set_*_json の失敗の種類を返す口を持つか (判断が要る)

## 目的

Godot の `set_effects_json`・`set_physics_json`・`set_base_physics_json` は、失敗すると `godot_error!` の文を出して false を返すだけで、
C ABI・wasm のような種類と補足の値 (`invalid_model`・`invalid_physics`・`errors` など) を返す口が無い (T0461 の候補)。
読み込みには `get_load_error()` (`{ "code", "error", ... }`) がある。

**判断が要る:** 口の形 (例: `get_load_error()` と同じ形の `get_last_error()` を足す・`set_*_json` の戻り値を辞書にする・今のまま文だけ)。
戻り値を変えると既存のスクリプトが動かなくなるので、足すだけの形を推奨とする見込み。C ABI・wasm の種類を揃える T0469 の後に着手する。

## 受け入れ条件

- [x] 口の形の案と推奨をメモに書き、利用者の判断を得た
- [x] 決めた形で、`set_*_json` の失敗の種類と補足の値を GDScript から読める。smoke (`smoke.gd`) か単体テストで確かめた
- [x] bindings/godot/README.md を直した
- [x] `just godot-check` と `just ci` が通る

## 参考

- T0461 のメモ (起票の候補)、bindings/godot/README.md (`get_load_error`)
- bindings/godot/src/node/sdk.rs

## メモ

- T0036 で起票した (元の候補: T0461 (Godot))。
- 判断 (2026-10-05、利用者の判断): `set_effects_json`・`set_physics_json`・`set_base_physics_json` の戻り値を辞書にする。リリース前なので移行の説明や互換の別名は要らない。README は今の形に即して書き直すだけ (利用者の指示)。
- やったこと:
  - `set_effects_json`・`set_physics_json`・`set_base_physics_json` (`bindings/godot/src/node/sdk.rs`) の戻り値を `VarDictionary` にした。成功は
    `{ "ok": true }`、失敗はエラーを出して `{ "ok": false, "code", "error", ... }` (`ok` のほかは `get_load_error()` と同じ形。`get_load_error()` は
    変えない)。種類はモデルが無い `no_model`・JSON を読めない `invalid_file`・エフェクトの検査の `invalid_model`・物理の検査の `invalid_physics`
    (どちらも SDK の `ugoe::validation_fields` で `errors`・`error_codes`。C ABI・wasm と同じ)・`set_base_physics_json` で合成できないときは
    `DressUpError::code` と `fields`。成否を `ok` のキーで見る形にした (空の辞書を成功にすると `if result:` が逆の意味になるため)。
  - 読み込みの `invalid_model` (`bindings/godot/src/model.rs` の `LoadError::info`) にも `ugoe::validation_fields` で `error_codes` を足した
    (T0469 の起票の候補)。単体テスト `describes_other_load_errors` で確かめた。
  - `example/tests/smoke.gd` で種類と補足の値 (`no_model`・`invalid_file`・`invalid_model` の `errors`・`error_codes`・`invalid_physics`) を確かめ、
    `example/tests/conformance.gd` の `set_effects_json` の呼び方を直した。example にほかに呼ぶスクリプトは無かった。
  - bindings/godot/README.md の表を直し、「設定の差し替えの結果」の節を足した (メモの最後の判断に従い、移行の説明は書いていない)。
    docs/portability.md のホストから来る値の表の Godot の列を直した。
  - `just godot-check` (smoke と適合テスト 69 件)・`just conformance`・`just ci` が通った。
- やらなかったこと: `set_base_physics_json` で合成に失敗する場合 (着せ替えの種類) の smoke は書いていない (起こすモデルの組が見本に無い)。
  コードは `get_load_error()` の着せ替えの失敗と同じ `field_variant` を使う。
