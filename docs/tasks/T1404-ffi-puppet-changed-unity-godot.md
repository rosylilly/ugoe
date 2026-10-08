---
id: T1404
title: "C ABI: ugoe_puppet_changed を足し、Unity・Godot でも形が変わらないフレームの書き換えを省く (判断が要る)"
status: todo
milestone: M8
crates: [ugoe-ffi, bindings/unity, bindings/godot, tests/conformance, docs]
depends_on: []
assignee:
---

# T1404: C ABI: ugoe_puppet_changed を足し、Unity・Godot でも形が変わらないフレームの書き換えを省く (判断が要る)

## 目的

T1402 で `ugoe` に `Puppet::changed()` (評価の出力を、最後に `true` を返したときの控えと許容誤差付きで比べる。docs/spec/evaluation.md#見た目の変化) を足し、
`<ugoe-avatar>` は形が変わらない間は描かなくなった。C ABI と、その上の Unity・Godot のバインディングにはこの口が無い。
C ABI に `ugoe_puppet_changed` を足し、Unity・Godot でも止まったモデルや収まりかけた物理のフレームで、頂点・色の書き換えを省く。

## 判断が要る

C ABI には既に描画可能要素ごとの変化のビット (`UgoeDrawableState::changes` の `UGOE_CHANGED_*`。直前の `update` との違い) があり、
Unity (`UgoePuppet.cs` の `upload`) と Godot (`render.rs`) はそれで転送を省いている。`changed` が足すのは「許容誤差を下回る違い (収まりかけた物理の揺れ) も
省く」ことと、「フレーム全体を 1 回で問える」ことだけ。

- **案 A (推奨): 足す。** `ugoe_puppet_changed(puppet, *mut bool) -> UgoeResult` (FFI の今の形に合わせる)。Unity・Godot は `false` のフレームで
  `ugoe_puppet_frame` の読み出しと書き換えを丸ごと省く (Unity は `Mesh` の書き換え、Godot は `mesh_surface_update_vertex_region`)。
  エンジンは毎フレーム描くので、省けるのは CPU の書き換えと転送だけ (画は前の頂点のバッファのまま)。
- 案 B: 足さない (要素ごとのビットで足りる)。ロードマップの「起票していない候補」に戻す。
- 名前: `ugoe_puppet_changed` (推奨。Rust の名前に合わせる) か、wasm に合わせた `ugoe_puppet_shape_changed` か。
  wasm の `shape_changed` はパラメータの書き換えで評価を待っている間 (`dirty`) も含むが、C ABI は `update` を呼ぶのがホストなので、`Puppet::changed` のままでよい見込み。

## 受け入れ条件

- [ ] 判断 (足すか・名前) をメモに書いた
- [ ] (A) `ugoe-ffi` に `ugoe_puppet_changed` を足し、ヘッダ (`include/`) を生成し直した。null・作成前などの扱いは既存の口に合わせた。単体テストで、
      止まったモデルの 2 回目の `update` の後は `false`、パラメータを変えた後は `true`、着せ替えの後は `true` になることを確かめた
- [ ] (A) Unity: C# の宣言を足し (`just refs` の宣言の過不足の検査が通る)、`false` のフレームで書き換えを省く。止まったモデルで `Mesh` を書き換えないことを
      テスト (tests/ の EditMode のテストか適合テスト) で確かめた
- [ ] (A) Godot: `false` のフレームで頂点のバッファを書き換えない。bindings/godot/README.md の検査が通る
- [ ] (A) 適合テスト (tests/conformance) に、C ABI の `changed` が `ugoe` と同じ並びを返すことを足した
- [ ] crates/ugoe-ffi/README.md#毎フレームの描画の状態、bindings/unity・bindings/godot の README を書き直した
- [ ] `just ci` が通る

## 参考

- T1402 のメモ (案 A を選んだ経緯・評価の出力で比べた理由・起票の候補)、docs/spec/evaluation.md#見た目の変化
- crates/ugoe-ffi/README.md#毎フレームの描画の状態 (T0490)、bindings/unity/com.ugoe.unity/Runtime/Unity/UgoePuppet.cs、bindings/godot/src/render.rs

## メモ

- T0078 で起票した (元の候補: T1402 (C ABI に `ugoe_puppet_changed` を足し、Unity・Godot でも形が変わらないフレームの描画を省く))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- Unity の実機のエディタでの確認は T0443 の手順に任せる (作業者の環境では Unity を動かせない。確かめた範囲をメモに書く)。
- 判断 (2026-10-08): 利用者が案 A (C ABI に ugoe_puppet_changed を足し、Unity・Godot でも変わらないフレームを省く) を選んだ。
