---
id: T1404
title: "C ABI: ugoe_puppet_changed を足し、Unity・Godot でも形が変わらないフレームの書き換えを省く (判断が要る)"
status: done
milestone: M8
crates: [ugoe-ffi, bindings/unity, bindings/godot, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T1404
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

- [x] 判断 (足すか・名前) をメモに書いた
- [x] (A) `ugoe-ffi` に `ugoe_puppet_changed` を足し、ヘッダ (`include/`) を生成し直した。null・作成前などの扱いは既存の口に合わせた。単体テストで、
      止まったモデルの 2 回目の `update` の後は `false`、パラメータを変えた後は `true`、着せ替えの後は `true` になることを確かめた
- [x] (A) Unity: C# の宣言を足し (`just refs` の宣言の過不足の検査が通る)、`false` のフレームで書き換えを省く。止まったモデルで `Mesh` を書き換えないことを
      テスト (tests/ の EditMode のテストか適合テスト) で確かめた
- [x] (A) Godot: `false` のフレームで頂点のバッファを書き換えない。bindings/godot/README.md の検査が通る
- [x] (A) 適合テスト (tests/conformance) に、C ABI の `changed` が `ugoe` と同じ並びを返すことを足した
- [x] crates/ugoe-ffi/README.md#毎フレームの描画の状態、bindings/unity・bindings/godot の README を書き直した
- [x] `just ci` が通る

## 参考

- T1402 のメモ (案 A を選んだ経緯・評価の出力で比べた理由・起票の候補)、docs/spec/evaluation.md#見た目の変化
- crates/ugoe-ffi/README.md#毎フレームの描画の状態 (T0490)、bindings/unity/com.ugoe.unity/Runtime/Unity/UgoePuppet.cs、bindings/godot/src/render.rs

## メモ

- T0078 で起票した (元の候補: T1402 (C ABI に `ugoe_puppet_changed` を足し、Unity・Godot でも形が変わらないフレームの描画を省く))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- Unity の実機のエディタでの確認は T0443 の手順に任せる (作業者の環境では Unity を動かせない。確かめた範囲をメモに書く)。
- 判断 (2026-10-08): 利用者が案 A (C ABI に ugoe_puppet_changed を足し、Unity・Godot でも変わらないフレームを省く) を選んだ。
- 名前は `ugoe_puppet_changed` (Rust の `Puppet::changed` に合わせた)。形は `ugoe_puppet_changed(const UgoePuppet *, bool *out, UgoeError *) -> UgoeResult`。
  wasm の `shape_changed` (描くまで積む・`dirty` を含む) とは意味が違うので名前を分けた。関数を足すだけなので `ABI_VERSION` は 3 のまま (README の規則)。
- やったこと:
  - `ugoe-ffi`: `src/frame.rs` に `ugoe_puppet_changed` (NULL は `UGOE_ERROR_NULL_POINTER`。`puppet_ref` と `write` で既存の口と同じ扱い)。
    ヘッダを生成し直した。`tests/frame.rs` に、止まったモデルの 2 回目の `update` の後は false・パラメータを変えた後は true・着せ替えの後は true、
    avatar を 120 フレーム進めて `ugoe::Puppet::changed` と同じ並び (true と false の両方を含む)、NULL を断ることのテスト。
    fuzz の入口 (`src/fuzzing.rs` の `frame`) からも呼ぶ。
  - **省いたフレームの後の書き換え:** 変更フラグは直前の `update` との違いなので、false のフレームを読まずに飛ばした後で、要素ごとのフラグだけを
    見て書き換えると、許容誤差を下回る違いが要素ごとに積もって (控えは true のたびに新しくなるので) 許容誤差を超えて残りうる。
    そのため Unity は飛ばした後はすべての要素の頂点・色を送り直し (`FrameSync.PulledAll`)、Godot は飛ばしたフレームの変更フラグを積んで足す
    (`PendingChanges`)。C ABI の README と docs/spec/evaluation.md#見た目の変化 にも書いた。
  - Unity: `Native.cs` に宣言、`Puppet.Changed()`、`FrameSync.PullIfChanged(force)`・`PulledAll`。`UgoePuppet.AfterEvaluate` は
    `false` で、ほかに送る理由 (`SyncRules.ForceSync`: バッチが未決定か `batching` が変わった・マスクの解像度が変わった・マスクのページが失われた)
    も無ければ、`ugoe_puppet_copy_frame` も `Mesh` の書き換えも中間のテクスチャの描き直しもしない (`FrameState.Evaluated` から
    中間のテクスチャの印を `FrameState.Sent` に分けた)。テストは `PuppetTests.UnchangedFramesAreNotPulled`・
    `ComponentStateTests.SyncIsForcedOnlyForReasonsOutsideTheShape`・`FrameStateTests.UnchangedEvaluationKeepsTheIntermediate`。
  - Godot: `Renderer::draw` は `Puppet::changed` が false で、最初の描画でもマスクの解像度の変化でもなければ何も書き換えずに戻る
    (頂点のバッファ・draw index・uniform)。`PendingChanges` の単体テスト (`cargo test --manifest-path bindings/godot/Cargo.toml`)。
  - 適合テスト: 結果に `changed` (各 `update`・`follow` の後の値) を足し、参照 (`Puppet::changed`) と C ABI (`ugoe_puppet_changed`)・
    Unity (`Puppet.Changed`) が完全に一致することを比べる (書かない wasm・Godot とは比べない)。計画には true と false の両方を含むケースがある
    (`avatar-physics`・`bundle-physics-pendulum`・`*-follow` など)。
- 確かめたこと: `just refs`・`just ffi-check`・`just conformance` (c・wasm・wasm-webgpu・avatar・queries・example・unity・godot すべて ok)・
  `just unity-check`・`just godot-check`・`just cross-check`・`just fuzz-lint`・`just ci`。
- 確かめていないこと: Unity の実機のエディタ (T0443 の手順に任せる。`UgoePuppet.cs` は `tests/UnityCompile` でコンパイルだけ確かめた)。
  Godot で実際にバッファを書き換えないこと (ヘッドレスの Godot の smoke と適合テストは通るが、書き換えの回数は数えていない)。
- 起票の候補:
  - Godot のノードに `is_shape_changed()` (あるいは描き直した回数) を出し、適合テストの `changed` と smoke で書き換えを省いたことを確かめる。
  - Godot の `draw` は従来、マスクの解像度の 1e-3 未満の変化を毎フレーム `mask_scale` に取り込んでいた (ゆっくりしたズームで閾値を超えない)。
    今は省いたフレームでは取り込まないので一部直ったが、描いたフレームでは同じ。Unity と同じく 2^(1/4) 刻みに丸めるか検討する。

