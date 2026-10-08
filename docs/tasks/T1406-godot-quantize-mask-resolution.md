---
id: T1406
title: "Godot: マスクの解像度を Unity と同じく 2^(1/4) 刻みに丸め、ゆっくりしたズームでもマスクを描き直す"
status: in-progress
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

- [ ] 案 A で、マスクの解像度を 2^(1/4) 刻みに丸めた。丸めの関数の単体テスト (`cargo test --manifest-path bindings/godot/Cargo.toml`) で、刻みの境目・
      同じ刻みの中の値が同じ解像度になること・0 や無限大を渡さないこと (node.rs の検査と合わせて) を確かめた
- [ ] 1 フレームに 0.05% ずつ 2 倍までズームすると、刻みを越えるたびにマスクを作り直すこと (作り直さない形では落ちること) を確かめた
      (render.rs の単体テストか、T1405 の `get_redraw_count` を使った `example/tests/smoke.gd`)
- [ ] bindings/godot/README.md#描画のしかた に丸めを書いた (Unity の README の書き方に合わせる)
- [ ] `just godot-check` と `just conformance --only godot` が通る (マスクのあるケースの画像が許容誤差の中に収まる。収まらなければ理由をメモに書く)
- [ ] `just ci` が通る

## 参考

- T1404 のメモ (起票の候補)、bindings/godot/src/render.rs (`Renderer::draw` の `rescale`・`MaskShrink`)・src/node.rs (`draw` の `screen_scale`)
- bindings/unity/com.ugoe.unity/Runtime/Unity/UgoePuppet.cs (`MaskResolution`)、docs/spec/rendering.md (マスク)

## メモ

- T0080 で起票した (元の候補: T1404 (Godot の draw はマスクの解像度の 1e-3 未満の変化も取り込み、ゆっくりしたズームでマスクを描き直さない。Unity と同じく 2^(1/4) 刻みに丸めるか検討する))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T1405 と同じ render.rs・node.rs を触り、T1405 の `get_redraw_count` で作り直しを数えられるので、T1405 の後にした。
