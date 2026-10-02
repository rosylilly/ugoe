---
id: T0133
title: ランタイム: 受け取ったパラメータの値を基準値として確実に書く口 (スタジオの受け手のため)
status: done
milestone: M8
crates: [ugoe-runtime, ugoe, ugoe-ffi, ugoe-wasm, bindings/web, docs, tests/conformance]
depends_on: [T0800]
assignee: claude-worker-T0133
---

# T0133: 受け取った値を基準値として書く口

## 目的

ADR 0028 の受け手 (出力ウィンドウ・ブラウザソース・仮想カメラ) は、指揮役から届いた値を基準値として書き、物理だけを自分で回す。
いまの Puppet の口では、モーション・表情・トラッキングの層と混ざらずに基準値を書けることが保証されていない (T0800 のメモ)。

## 受け入れ条件

- [x] Puppet に、届いた値を基準値として書き、自分のモーション・表情・トラッキングの層を通さずに物理だけを進める口を足す (既定の挙動は変えない)
- [x] C ABI と wasm に出す。WebPuppet にはパラメータを Float32Array でまとめて書く口も足す (T0808 の負荷を下げる)
- [x] 同じ {dt, values} の列を与えた 2 つの Puppet (ネイティブと wasm) の頂点が許容誤差内で一致するテスト
- [x] `just ci`・`just wasm-check`・`just ffi-check`・`just conformance --only c,wasm` が通る

## 参考

- docs/adr/0028-studio.md、docs/studio.md

## メモ

- **やったこと:**
  - `Puppet::set_base_values(&[f32])` (定義順の値をすべて基準値として書く。前のフレームの出力とビット列で同じでも基準値になる) と
    `Puppet::follow(&[f32], dt)` (基準値として書き、物理・ポーズ・評価だけを行う。モーション・トラッキング・表情・目パチ・呼吸・
    リップシンクは通さず、時刻も進めない) を `crates/ugoe/src/follow.rs` に足した。数の違いと NaN は `ParameterValuesError` で、何も変えない。
    `update` は変えていない (`apply_part_opacities` にモーションのカーブを使うかの引数を足しただけ)。
  - C ABI: `ugoe_puppet_set_base_values`・`ugoe_puppet_follow` (ヘッダは cbindgen の CLI で作り直した)。wasm: `WebPuppet` の
    `set_base_values`・`follow`・`parameter_values` (`Float32Array`)。`bindings/web` のスタブと README、各クレートの README を揃えた。
  - `docs/spec/motion.md#受け手として進める` を足し、`docs/studio.md` の受け手の手順を `Puppet::follow` にした。
    公開仕様 (`spec/`) は変えていない (適用順は同じで、Puppet の API を足しただけ。T0416 と同じ判断)。
  - テスト: `crates/ugoe/src/follow.rs` (物理を切った指揮役 + 受け手が、物理を有効にした 1 つの Puppet と許容誤差内で一致。静的と動的の物理)、
    `crates/ugoe-ffi/tests/extended.rs` (C ABI と `Puppet::follow` の一致、受け付けない入力)。
  - 適合テスト (team-lead の承認を得て `tests/conformance` を範囲に足した。wasm を実際に動かして比べられるのがここだけのため):
    `follow` のケース (`avatar-follow`・`dress-up-follow`) を足した。計画は物理を切った指揮役 (標準の目パチと呼吸) の各フレームの値を持ち、
    C と wasm の実行器は `follow` で進め、参照は `Puppet::follow`。目パチと呼吸は指揮役だけにあるので、`follow` を呼ばずに `update` で
    進めた実行器は合わない (単体テストで確かめた)。`just conformance --only c,wasm` で 2 ケースとも c・wasm の両方で比べて通った (55 ケース)。
- **確かめていないこと:** WebGPU の wasm (`wasm-webgpu`) と Unity・Godot での実行。エディタとビューアの wasm のスタブ (使うものだけを書く) は変えていない。
- **制限:** リップシンクの対象を物理の入力にするモデルでは、指揮役 + 受け手は 1 つの Puppet と一致しない (受け手の物理はリップシンク後の値を読む)。
  `docs/spec/motion.md` に書いた。`follow` の後の `motion_events` は直前の `update` のまま。
- **後続の候補:** Unity (`bindings/unity`) と Godot (`bindings/godot`) に同じ口を出し、適合テストの `FOLLOW_BINDINGS` に足す
  (今はその 2 つで `follow` のケースを飛ばす)。スタジオの受け手 (T0801・T0804・T0808) はこの口を使う。

