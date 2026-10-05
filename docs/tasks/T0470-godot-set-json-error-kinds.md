---
id: T0470
title: "Godot: set_*_json の失敗の種類を返す口を持つか (判断が要る)"
status: in-progress
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

- [ ] 口の形の案と推奨をメモに書き、利用者の判断を得た
- [ ] 決めた形で、`set_*_json` の失敗の種類と補足の値を GDScript から読める。smoke (`smoke.gd`) か単体テストで確かめた
- [ ] bindings/godot/README.md を直した
- [ ] `just godot-check` と `just ci` が通る

## 参考

- T0461 のメモ (起票の候補)、bindings/godot/README.md (`get_load_error`)
- bindings/godot/src/node/sdk.rs

## メモ

- T0036 で起票した (元の候補: T0461 (Godot))。
- 判断 (2026-10-05、利用者の判断): `set_effects_json`・`set_physics_json`・`set_base_physics_json` の戻り値を辞書にする (既存のスクリプトの bool の判定は変わる。README に移行の書き方を書く)。
