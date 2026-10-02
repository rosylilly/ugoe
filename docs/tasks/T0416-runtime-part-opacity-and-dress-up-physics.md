---
id: T0416
title: SDK: 評価後のパーツの不透明度を公開する・着せ替え中に本体の物理を差し替える
status: done
milestone: M7
crates: [ugoe-runtime, ugoe, ugoe-ffi, ugoe-wasm, docs, bindings/web]
depends_on: [T0414]
assignee: claude-worker-T0416
---

# T0416: SDK の残り (T0414 の後続)

## 目的

T0414 の後続の候補をまとめる。

## 受け入れ条件

- [x] 評価後のパーツの不透明度 (祖先を掛けた値) を ugoe-runtime から公開し、C ABI と wasm に出す (今の C ABI の part_opacity は、直前の update が書いた上書きの値)
- [x] 着せ替え中に本体の物理の設定を差し替える SDK の API (今は付け外しのたびに本体の設定から作り直すので失われる)。仕様 (docs/spec/modules.md と spec/) との関係を決める
- 黒箱の比較で view を渡すのは、spec/ だけを読む作業者の担当なので T0422 に分けた
- [x] `just ci`・`just wasm-check`・`just ffi-check` が通る

## 参考

- `docs/tasks/T0414-host-actions-in-bindings.md` のメモ

## メモ

- **評価後のパーツの不透明度:** `ModelInstance::part_opacity(part) -> f32` (上書きを当てた自身の値 × 祖先。直前の `update` の結果)。
  C ABI は `ugoe_puppet_part_evaluated_opacity(puppet, index, *out f32)` (既存の `ugoe_puppet_part_opacity` は上書きの値のまま。名前を変えると
  ABI が壊れるので別の名前にした)。wasm は `part_evaluated_opacity(part: string) -> number | undefined` (必要なら評価してから読む)。
  docs/spec/evaluation.md の 2. に一行。
- **着せ替え中の本体の物理:** `Puppet::set_base_physics(Option<PhysicsSettings>) -> Result<(), DressUpError>`。本体のモデルに対して検査し、
  付けていれば今のモジュールの組と合成し直して (`set_modules` と同じ `compose_all` を共有)、合成した設定も検査する。失敗は何も変えない
  (検査は `DressUpError::Physics`)。付けたことがなければ `set_physics` と同じ。本体の物理の有無が変わってスロットの `colliders` を空にするか
  (`compose_physics_without_base`) が変わるときだけモデルを差し替え (`swap_model`)、そうでなければ物理だけを差し替える。
  C ABI は `ugoe_puppet_set_base_physics_json` (検査は `UGOE_ERROR_INVALID_MODEL`、合成は `UGOE_ERROR_DRESS_UP`。モデルを差し替えたときだけ
  `refresh_model`) と `ugoe_puppet_base_physics_json`、wasm は `set_base_physics_json` / `base_physics_json`。`set_physics` (と `_json`) は今のまま
  (合成した設定を差し替え、次の付け外しで失われる)。
- **仕様との関係 (決めたこと):** 英語の公開仕様は変えない。spec/model.md#composition は「本体の物理の設定とモジュールの組から合成した設定」を
  決めるだけで、どれを本体とするかや差し替えの API は決めていない (`Puppet` の API は公開仕様に含めない。docs/spec/public-spec.md の Composition の行)。
  `set_base_physics` は合成の入力を差し替えるだけなので食い違わない。docs/spec/modules.md#puppet-での着せ替え に 2 つの差し替えの違いと
  このことを書いた。`just spec` は spec/ を変えていないので回していない。
- **範囲外:** `bindings/web/stub/ugoe_wasm.d.ts` に 3 行 (承認済み。crates: に bindings/web を足した)。生成物 (`just wasm` の scratchpad 出力) と照合した。
- **テスト:** runtime `part_opacity_reads_the_evaluated_value`、SDK `base_physics_replaced_while_dressed_survives_dress_up`・
  `base_physics_toggles_exposed_colliders_while_dressed`・`invalid_base_physics_changes_nothing`、ffi `evaluated_part_opacity_follows_the_puppet` (host.rs)・
  `base_physics_survives_dress_up` (extended.rs)、wasm `part_evaluated_opacity_reads_the_evaluated_value`・`base_physics_json_survives_dress_up`。
  C の見本でも新しい関数を呼ぶ。
- **確認:** `just ci`・`just wasm-check` (main 6931c0b に rebase 後)、`just ffi-check`・`just conformance --only c,wasm` (rebase 前。rebase で入ったのは
  tools/spec-check だけ)。`just test-gpu` は描画と構造体を変えていないので回していない。
- **確かめていないこと:** Windows と macOS での C の見本。ビューア・エディタの画面 (API を足しただけで呼んでいない)。
- **後続の候補:**
  - bindings/unity (`Native.cs`・`Puppet.cs`) と bindings/godot に `part_evaluated_opacity` と `set_base_physics_json` を出す。
  - wasm にもパーツの不透明度の上書き (`part_opacity_override`) を読む API (T0414 の候補の残り。今回は評価後の値だけ)。
  - エディタのプレビューで、着せ替え中に物理を編集したら `set_base_physics_json` を使う (今は `set_physics_json`)。
