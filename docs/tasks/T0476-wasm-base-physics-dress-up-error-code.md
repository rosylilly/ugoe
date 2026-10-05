---
id: T0476
title: "wasm: set_base_physics_json の合成の失敗に着せ替えの種類と補足の値を付ける (判断が要る)"
status: todo
milestone: M9
crates: [ugoe-wasm, bindings/web, docs]
depends_on: []
assignee:
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

- [ ] 案と推奨をメモに書き、利用者の判断を得た
- [ ] 決めた形で `set_base_physics_json` の合成の失敗に `code` と補足の値が付く。ugoe-wasm のテストで、合成できないモジュールを付けた状態で確かめる
- [ ] `codes.json` (と `src/codes.rs` の網羅の見本)・`@ugoe/runtime` の型とテスト (`codes.test.ts`)・bindings/web/README.md・crates/ugoe-wasm/README.md を直した
- [ ] docs/portability.md のホストから来る値の表の wasm の列を直した
- [ ] `just wasm-check`・`just conformance`・`just ci` が通る

## 参考

- T0469 のメモ (やらなかったこと・起票の候補)、T0470 のメモ (Godot の形)
- crates/ugoe-wasm/src/codes.rs・src/animation.rs、bindings/web/src/settings-errors.ts

## メモ

- T0038 で起票した (元の候補: T0469 (wasm の合成の失敗))。
