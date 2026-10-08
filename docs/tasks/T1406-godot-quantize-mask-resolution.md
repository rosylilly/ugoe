---
id: T1406
title: "Godot: マスクの解像度を Unity と同じく 2^(1/4) 刻みに丸め、ゆっくりしたズームでもマスクを描き直す"
status: done
milestone: M8
crates: [bindings/godot, docs]
depends_on: [T1405]
assignee: claude-worker-T1406
---

# T1406: Godot: マスクの解像度を 2^(1/4) 刻みに丸め、ゆっくりしたズームでもマスクを描き直す

## 目的

Godot の `Renderer::draw` (bindings/godot/src/render.rs) は、マスクの解像度 (`mask_scale` に画面上の倍率を掛けたもの) が前の値から 1e-3 (0.1%) を超えて
変わったときだけマスクを作り直す (`rescale`)。ところが前の値 (`self.mask_scale`) を描いたフレームごとに今の値で上書きするので、1 フレームの変化が 0.1% 未満の
ゆっくりしたズーム (カメラのズームのアニメーションなど) では、合わせて何倍に変わっても作り直さず、マスクが粗いまま (か、無駄に細かいまま) になる
(前からある問題。T1404 で省いたフレームでは取り込まなくなったが、描いたフレームでは同じ。T1404 のメモ)。
Unity (`UgoePuppet.MaskResolution`) は解像度を 2^(1/4) (約 19%) 刻みに切り上げて丸め、刻みが変わったときだけ作り直す。Godot も同じにする。

## 案と推奨

- **案 A (推奨): Unity と同じく 2^(1/4) 刻みに切り上げて丸める。** `2^(ceil(log2(s) * 4) / 4)`。丸めた値が変わったときだけ作り直す
  (`1e-3` の比較は要らなくなる)。マスクの画素は最大で約 19% 多くなるが、粗くはならない。Unity と描き方がそろう。範囲 (Unity は 1/16〜16 で切る) も合わせるか、
  今の Godot の下限 (`mask_scale` の 0.01) との違いをメモに書いて決める。
- 案 B: 丸めず、前の値を作り直したときだけ更新する (作り直した時の値から 0.1% を超えたら作り直す)。ゆっくりしたズームでも直るが、
  ズームの間はほぼ毎フレーム作り直す (Unity の丸めがこれを避けている)。

どちらもマスクの解像度の決め方だけで、公開の口は変えない。

## 受け入れ条件

- [x] 案 A で、マスクの解像度を 2^(1/4) 刻みに丸めた。丸めの関数の単体テスト (`cargo test --manifest-path bindings/godot/Cargo.toml`) で、刻みの境目・
      同じ刻みの中の値が同じ解像度になること・0 や無限大を渡さないこと (node.rs の検査と合わせて) を確かめた
- [x] 1 フレームに 0.05% ずつ 2 倍までズームすると、刻みを越えるたびにマスクを作り直すこと (作り直さない形では落ちること) を確かめた
      (render.rs の単体テストか、T1405 の `get_redraw_count` を使った `example/tests/smoke.gd`)
- [x] bindings/godot/README.md#描画のしかた に丸めを書いた (Unity の README の書き方に合わせる)
- [x] `just godot-check` と `just conformance --only godot` が通る (マスクのあるケースの画像が許容誤差の中に収まる。収まらなければ理由をメモに書く)
- [x] `just ci` が通る

## 参考

- T1404 のメモ (起票の候補)、bindings/godot/src/render.rs (`Renderer::draw` の `rescale`・`MaskShrink`)・src/node.rs (`draw` の `screen_scale`)
- bindings/unity/com.ugoe.unity/Runtime/Unity/UgoePuppet.cs (`MaskResolution`)、docs/spec/rendering.md (マスク)

## メモ

- T0080 で起票した (元の候補: T1404 (Godot の draw はマスクの解像度の 1e-3 未満の変化も取り込み、ゆっくりしたズームでマスクを描き直さない。Unity と同じく 2^(1/4) 刻みに丸めるか検討する))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T1405 と同じ render.rs・node.rs を触り、T1405 の `get_redraw_count` で作り直しを数えられるので、T1405 の後にした。
- T1406 でやったこと (案 A):
  - `src/render.rs` に `quantize_mask_scale` (1/16〜16 に収めてから `2^(ceil(log2(s) * 4) / 4)`。NaN は 1) と、丸めた値を覚えて
    変わったら true を返す `MaskResolution` を足し、`Renderer::draw` の `1e-3` の比較を置き換えた。丸めた値が変わったら必ず描くので、
    描かないフレームに取り込んでも前の値は失わない。マスクの大きさ・`MaskShrink` には丸めた値を渡す。
  - 範囲は Unity に合わせて 1/16〜16 にした (`mask_scale` に画面上の大きさを掛けた値に掛ける)。`mask_scale` のプロパティの下限 0.01 は
    そのまま (掛けた後で 1/16 に切り上がる)。0 (画面上の大きさが 0)・負・無限大・NaN は丸めの関数が有限で正の値にする
    (node.rs の `set_mask_scale` は有限だけを受け付けるが、画面上の大きさは検査していないため、丸めの側で受ける)。
    上限 16 で、smoke.gd の `mask_scale = 40` は 16 で描くことになる (アトラスの 4096 には `MaskArea::fit` と `MaskShrink` が収める)。
  - 単体テスト: 刻みの境目・同じ刻みの中の値・0/負/無限大/NaN (`mask_scale_is_quantized_to_quarter_octaves`)、1 フレームに 0.05% ずつ
    2 倍までズームすると 4 回作り直し、前の形 (前のフレームと 0.1% を比べる) では 0 回 (`slow_zoom_rescales_at_each_step`)。
    `example/tests/smoke.gd` でも `get_redraw_count()` で同じズームが 4 回描き直すことを確かめた。
  - README の「描画のしかた」に丸めを書き (Unity の README には丸めの記述が無かったので、Godot の README の書き方で書いた)、
    `mask_scale` の表と `MaskShrink` の「測り直す」条件を直した。
  - 検査: `cargo test --manifest-path bindings/godot/Cargo.toml`・`just godot-check` (69 ケース。マスクのケースは 100% 一致、最大 2/255)・
    `just conformance --only godot`・`node scripts/check-licenses.mjs --godot`・`just ci` が通った。
- 起票の候補: なし。
