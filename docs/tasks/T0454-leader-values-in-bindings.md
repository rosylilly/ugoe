---
id: T0454
title: 指揮役の側の値 (物理の直前の値・当てたリップシンク・パーツの不透明度) を C ABI・wasm・Godot から読む
status: done
milestone: M9
crates: [ugoe-ffi, ugoe-wasm, bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0454
---

# T0454: 指揮役の側の値 (物理の直前の値・当てたリップシンク・パーツの不透明度) を C ABI・wasm・Godot から読む

## 目的

スタジオの指揮役と受け手 (ADR 0028) では、指揮役が `Puppet::values_before_physics` などを受け手へ配る。SDK (`ugoe` の `follow.rs`) には
読む口があるが、C ABI・wasm・Godot には無い (受け手の `follow` だけがある。T0449 で Godot を SDK に揃えたときに分かった)。
組み込み先のエンジンでも指揮役になれるよう、3 つで同じ名前と形で足す。

**判断が要る:** C ABI・wasm・Godot の公開 API の追加。名前 (`get_values_before_physics`・`get_lip_sync_applied`・`get_part_opacities_applied` など) と、
配列の返し方 (C ABI は呼び出し側の領域へ写すか、ポインタと長さか) を、既存の口 (パラメータの値の読み出し) に揃えて決める。

## 受け入れ条件

- [x] 名前と形を決め、3 つの組み込み先に足した (doc コメント・README)
- [x] 適合テスト (`tests/conformance`) に、指揮役の値を読んで別の Puppet に `follow` させると参照と一致するケースがある (または既存の `*-follow` のケースを指揮役の側の口で組み直す)
- [x] `just ci`・`just wasm-check`・`just ffi-check`・`just godot-check`・`just conformance` が通る

## 参考

- T0449 のメモ (起票の候補)
- crates/ugoe/src/follow.rs、ADR 0028

## メモ

- T0019 で起票した (元の候補: T0449 の指揮役の口)。

### T0454 の作業 (やったこと)

- 名前と形 (どれも SDK の名前に、そのバインディングの流儀で揃え、そのバインディングの受け手の口 `follow` にそのまま渡せる形にした):
  - C ABI: `ugoe_puppet_values_before_physics(puppet, out, capacity, out_count)`・`ugoe_puppet_lip_sync_applied(puppet, out_level, out_strength)`・
    `ugoe_puppet_part_opacities_applied(puppet, out, capacity, out_count)` (`UgoePartOpacityInput` の並び)。並びは **呼び出し側の領域へ写す**
    (`ugoe_puppet_tracking_targets`・`ugoe_puppet_hit_areas_at` と同じ。足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で数だけ書く)。
    ポインタと長さで借りる形にしなかったのは、指揮役は毎フレーム値を送り出すのでコピーは避けられず、`update` で無効になるポインタの寿命の約束を
    増やさないため。リップシンクを当てていなければ音量 -1・強さ 1 (`ugoe_puppet_follow` は負の音量を「当てない」とみなす)。
  - wasm: `values_before_physics(): Float32Array`・`lip_sync_applied(): Float32Array | undefined` (`[音量, 強さ]`。bindings/web の README の
    `lipSync?.[0]` の形)・`part_opacities_applied_parts(): Uint32Array`・`part_opacities_applied_values(): Float32Array` (値と重みを交互に)。
  - Godot: `get_values_before_physics() -> PackedFloat32Array`・`get_lip_sync_applied() -> Vector2` (当てなければ `(-1, 1)`)・
    `get_part_opacities_applied() -> Array` (`{part, value, weight}`。`follow` の引数が型の無い `Array` なので、`Array[Dictionary]` では
    渡せず (godot-rust が呼び出しを断る)、型の無い配列で返す)。
  - **Unity にも出した** (C ABI の薄い層なので): `Puppet.ValuesBeforePhysics()`・`Puppet.LipSyncApplied()` (`LipSyncInput?`。型を
    `Types.cs` に足した)・`Puppet.PartOpacitiesApplied()`。`Native.cs` に 3 つの宣言を足したので `check-ffi-bindings.mjs` の OMIT は要らない。
  - `follow` の後に読むと、受け手に渡した値 (値域にクランプしたもの) を返す (SDK の `follow` がそう記録するため。受け手から中継できる) ことを doc に書いた。
- 適合テスト: 計画に `conduct` (`Case::conduct`。`cases.json` の `"conduct": true`) を足し、`avatar-conduct`・`lip-sync-conduct`・
  `part-switch-conduct` を追加した。実行器 (C・wasm (WebGL2/WebGPU)・Unity・Godot) は計画の `follow` の値を読まずに、実行器の中で指揮役を
  作って `update` で進め (効果の設定の JSON・モーション・フレームごとのリップシンクの音量は計画の `conduct` から)、指揮役の口で読んだ値で受け手を
  `follow` させる。参照は計画の `follow` の値で進めるので、口が返す値と渡す形の両方を確かめる。既存の `*-follow` は値を運ぶ形の検査として残した。
  Rust 側にも同じ手順を SDK で行うと参照と一致するテスト (`conduct_cases_match_the_planned_follow_values`) を置いた。
- テスト: `crates/ugoe-ffi/tests/extended.rs` (SDK と同じ値・受け手の駆動・BUFFER_TOO_SMALL・NULL)、`bindings/unity/tests/ReceiverTests.cs`、
  `bindings/godot/example/tests/smoke.gd`。
- 文書: 各 README (ffi・wasm・web・godot (「制限」の項を消した)・unity)、`docs/spec/motion.md#受け手として進める`、`tests/conformance/README.md`。
  公開仕様 (`spec/`) は変えない (API の口を足しただけ。T0133〜T0135 の判断と同じ)。
- 範囲外を少し触った (小さなものなので行った): `bindings/unity` (Unity の口)、`bindings/web` (`stub/ugoe_wasm.d.ts` の型の写しと README)、
  `tests/conformance` (受け入れ条件の適合テスト)。
- 確かめたこと: `just ci`・`just wasm-check`・`just ffi-check`・`just godot-check` (Godot 4.7.1・Xvfb)・`just conformance`
  (c・wasm・wasm-webgpu・unity・godot がすべて 63 ケース合格)・`just unity-check` が通る (main に rebase した後)。
  Windows・macOS・実際の Unity エディタでは確かめていない。

### 起票の候補

- Godot の `set_lip_sync_level` は負の値でも 0 にクランプして入力をやめられない (C ABI・wasm・Unity は負か null でやめられる)。
  入力をやめる口 (`clear_lip_sync_level` など) を足すか。
- bindings/web の `<ugoe-avatar>` やプレイヤーで指揮役になる用途があれば、高い層の API (フレームを作る関数など) を考える。

