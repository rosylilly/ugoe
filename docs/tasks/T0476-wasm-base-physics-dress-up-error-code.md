---
id: T0476
title: "wasm: set_base_physics_json の合成の失敗に着せ替えの種類と補足の値を付ける (判断が要る)"
status: done
milestone: M9
crates: [ugoe-wasm, bindings/web, docs]
depends_on: []
assignee: claude-worker-T0476
---

# T0476: wasm: set_base_physics_json の合成の失敗に着せ替えの種類と補足の値を付ける (判断が要る)

## 目的

T0469 で、wasm の `set_physics_json`・`set_base_physics_json` の検査の失敗は `name` が `"Error"` の coded error (`invalid_physics`) を投げるようになった。
ところが `set_base_physics_json` で付けているモジュールと合成し直せないとき (`AnimationError::DressUp`) の例外は `code` を持たない。
C ABI は `UGOE_ERROR_DRESS_UP` と着せ替えの種類を返し、Godot は T0470 の判断で `DressUpError::code` と `fields` を返す (T0469 の候補)。

**判断が要る:** 例外の `name`。案:

- A. 付け外しの失敗と同じ `name` = `"DressUpError"`・`code` = `DressUpError::code`・補足の値 (C ABI・Godot と同じ種類)。`@ugoe/runtime` の
  `isDressUpError` でそのまま見分けられる。**推奨** (同じ原因の失敗が同じ形になる)。
- B. 設定の差し替えの失敗と同じ `name` = `"Error"` で、`code` に着せ替えの種類を入れる (`isSettingsError` が知らない `code` を受けるかを決め直す)。

## 受け入れ条件

- [x] 案と推奨をメモに書き、利用者の判断を得た
- [ ] 決めた形で `set_base_physics_json` の合成の失敗に `code` と補足の値が付く。ugoe-wasm のテストで、合成できないモジュールを付けた状態で確かめる
- [x] `codes.json` (と `src/codes.rs` の網羅の見本)・`@ugoe/runtime` の型とテスト (`codes.test.ts`)・bindings/web/README.md・crates/ugoe-wasm/README.md を直した
- [x] docs/portability.md のホストから来る値の表の wasm の列を直した
- [x] `just wasm-check`・`just conformance`・`just ci` が通る

## 参考

- T0469 のメモ (やらなかったこと・起票の候補)、T0470 のメモ (Godot の形)
- crates/ugoe-wasm/src/codes.rs・src/animation.rs、bindings/web/src/settings-errors.ts

## メモ

- T0038 で起票した (元の候補: T0469 (wasm の合成の失敗))。
- 判断 (2026-10-06、利用者の判断): 案 A。`name` = `"DressUpError"`、`code` と補足の値は着せ替えの失敗と同じ (C ABI・Godot と同じ種類)。
- やったこと:
  - `AnimationError::name` を足し、合成の失敗 (`AnimationError::DressUp`) は付け外しの失敗と同じ `"DressUpError"`、ほかは `"Error"` にした。
    `AnimationError::info` は合成の失敗に、付け外しの失敗と同じ `code` と補足の値 (`dress_up::dress_up_info`。`error_info` から切り出した。
    SDK の `DressUpError::code`・`fields` で C ABI・Godot と同じ) を返す。`src/web/animation.rs` の `animation_error` は `name` をそこから取る。
  - `src/codes.rs`: `AnimationError` の見本のうち `name` が `"DressUpError"` のものは `dress_up_error` に入れる。種類はすべて付け外しの失敗の
    見本に既にあるので、`codes.json` の中身は変わらない (`settings_error` に `not_attached` が混ざらないことをテストが確かめる)。
  - `@ugoe/runtime`: `isSettingsError` は `name` が `"Error"` のものだけを受けるようにした (同じ `code` の `invalid_physics` を持つ
    `DressUpError` を受けないため)。`UgoeSettingsError` に `name: "Error"`。`settings-errors.test.ts` に合成の失敗を `isDressUpError` が受け、
    `isSettingsError` が受けないテストを足し、`warnings.ts`・`codes.test.ts` のコメントを直した。
  - crates/ugoe-wasm/README.md・bindings/web/README.md・docs/portability.md (wasm の列) を直した。
  - `just wasm-check`・`just wasm-types`・`just conformance`・`just ci` が通った (rebase 後)。
- やらなかったこと: 受け入れ条件 2 の「合成できないモジュールを付けた状態で」のテストは書けなかった。今の SDK の `Puppet::set_base_physics` は、
  本体の設定を本体のモデルで検査してから合成するので、合成が失敗する組を作れない (`attach` と `compose` は付けたときと同じ入力で必ず通り、
  `compose_physics` の `UnknownCollider` はスロットが公開する衝突体が本体の設定に無いときだけだが、それは先の検査が `unknown_reference` で弾く。
  合成した設定の検査の失敗は `DressUpError::Physics` で `invalid_physics` (今までどおり `"Error"`) になる)。代わりに `compose_failure_is_a_dress_up_error`
  で、SDK の `DressUpError` (`unknown_collider`・`compile`) から作った `AnimationError::DressUp` の `name`・`code`・補足の値が付け外しの失敗
  (`error_info`) と同じことを確かめた。T0470 の Godot も同じ理由で smoke を書いていない。
- 起票の候補: なし。
