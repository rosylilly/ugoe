---
id: T0461
title: エフェクトの設定を差し替える set_effects_json (C ABI・wasm・Godot) で設定を検証する
status: done
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0461
---

# T0461: エフェクトの設定を差し替える set_effects_json (C ABI・wasm・Godot) で設定を検証する

## 目的

目パチ・呼吸・リップシンクの設定を差し替える `set_effects_json` (C ABI・wasm・Godot) は設定を検証しない。JSON の `1e39` は `f32` の無限大になる。
ランタイムは安全に扱う (docs/portability.md#ホストから来る数 の表) が、作者の誤りを見逃す (T0010 の候補)。モデルの読み込みと同じ検証を通す。

今まで受けていた入力を拒むようになるので、拒むときのエラー (種類と補足の値) を各組み込み先で揃える。

## 受け入れ条件

- [x] SDK に設定の検証 (モデルの `validate` のエフェクトの検査と同じもの) を通す入口があり、C ABI・wasm・Godot がそれを使う
- [x] 有限でない値・範囲の外の値を拒み、拒んだときに今の設定を変えないテストがある (SDK・C ABI・wasm)
- [x] 各 README と docs/portability.md を更新した
- [x] `just ci`・`just wasm-check`・`just godot-check` が通る

## 参考

- T0010 のメモ (T0010 の結果の起票の候補)
- crates/ugoe-ffi/src/animation.rs、crates/ugoe-wasm/src/animation.rs、bindings/godot/src/node/sdk.rs

## メモ

- T0026 で起票した (元の候補: T0010 の `set_effects_json`)。
- **やったこと (2026-10-05):**
  - `ugoe_model::validate::validate_effects(&effects, &model)` を足した (`validate` のエフェクトの検査をそのまま切り出したもの。
    対象パラメータの参照・時間と周期の範囲・有限値)。`ugoe-model` は `crates:` の外だが、`validate_physics` と同じ形の追加だけなので行った。
    `docs/spec/model.md#検証` に書いた。
  - SDK に `Puppet::try_set_effects` (検査して通れば `set_effects`、通らなければ何も変えずに `Vec<ValidationError>`) を足した。`set_effects` は今のまま検査しない。
  - C ABI の `ugoe_puppet_set_effects_json`: 検査を通らなければ `UGOE_ERROR_INVALID_MODEL`、`ugoe_last_error_kind` は `invalid_model`、
    `ugoe_last_error_field("errors")` は検査のエラーの数 (読み込みの `invalid_model` と同じ種類と補足の値)。文は `describe_errors("effects", …)`。
  - wasm の `set_effects_json`: 同じ文で、`code` が `invalid_model`・`errors` が数の `Error` (`name` は `"Error"`) を投げる (`AnimationError::InvalidEffects`・`info`)。
  - Godot の `set_effects_json`: 同じ文の `godot_error!` を出して false (Godot の `set_*` は種類を返す口が無いので、文だけ揃えた)。
  - テスト: SDK (`try_set_effects_rejects_invalid_effects_and_keeps_the_current_ones`)・`ugoe-model` (`validate_effects_checks_the_given_effects`)・
    C ABI (`tests/host.rs` の `invalid_effect_settings_are_rejected_without_changes`)・wasm (`invalid_effects_json_is_rejected_without_changes`)・
    Godot の smoke (`1e39` とモデルにないパラメータを拒み、リップシンクが前の設定のまま)。
  - 文書: `crates/ugoe-ffi/README.md`・`crates/ugoe-wasm/README.md`・`bindings/godot/README.md`・`docs/portability.md`・`docs/spec/motion.md`・
    英語の `spec/motion.md#replacing-the-effect-settings` (ランタイムが検査してよく、拒むなら今の設定を変えない、を MAY/MUST で)。
  - `crates:` の外の小さな変更: `tests/conformance/src/reference.rs` の指揮役のエフェクト (標準の目パチと呼吸) から、モデルにないパラメータを外した
    (`lip-sync-conduct` のモデルに `EyeLOpen` などが無く、`set_effects_json` が拒むようになったため。ランタイムはもともと無視していたので結果は同じ)。
    `bindings/unity` の `Puppet.SetEffectsJson` の doc コメントに `InvalidModel` を書いた。
- **注意 (振る舞いの変更):** 参照の検査も入れたので (タスクの「モデルの `validate` のエフェクトの検査と同じもの」のとおり)、`Effects::standard()` を
  標準パラメータを持たないモデルへ `set_effects_json` で渡すと拒まれるようになった (今までは無いパラメータを無視して動いた)。
  数だけを検査して参照は見ない案もあったが、物理 (`set_physics_json`) とエディタの `set_effects` が参照を検査するのに合わせた。
- **確かめたこと:** `just ci`・`just wasm-check`・`just godot-check` (63/63 と smoke)・`just conformance` (C・wasm・wasm-webgpu・avatar・queries・Unity・Godot すべて 63/63 など)・
  `just refs`・`just unity-check` が通った (rebase 前に conformance・godot-check、rebase 後に ci・wasm-check)。実際の Unity エディタでは確かめていない。
- **起票の候補:**
  - `@ugoe/runtime` (`bindings/web`) の型と README に、`set_effects_json` の例外 (`code: "invalid_model"`・`errors`) を書く。範囲の外なので触っていない。
  - Godot の `set_*_json` が失敗の種類を返す口 (読み込みの `load_error` のような辞書) を持つか。今は文だけ。

