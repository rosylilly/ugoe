---
id: T0467
title: "Godot: bench.gd の計測・Forward+ の経路の適合テスト・BC1 の 3 色の形のケース"
status: todo
milestone: M9
crates: [bindings/godot, tests/conformance, docs]
depends_on: []
assignee:
---

# T0467: Godot: bench.gd の計測・Forward+ の経路の適合テスト・BC1 の 3 色の形のケース

## 目的

Godot の GDExtension の検査に、環境が要るために残したものがある (T0460・T0223 の候補)。

1. T0460 は機械が空かず、`bench.gd` (描画を頂点だけの書き換えにするかの計測) を静かな機械か CI の `godot` のジョブで流していない。
2. 適合テストは opengl3 の経路だけで流している。RenderingDevice の経路 (Forward+。GPU のある機械か lavapipe が要る) でも流せるようにする。
3. BC1 のアルファなしの形式で、3 色のモードの 4 つ目の画素は Godot では透明になるはず (参照は不透明)。適合テストの書き出しは 3 色のモードを使わないので確かめていない。
   3 色のモードを使う BC1 のケースを足し、違えば Godot の側でアルファを 255 にする。

## 受け入れ条件

- [ ] 静かな機械か CI の `godot` のジョブで `bench.gd` を流し、時間を bindings/godot の README か docs に記録した
- [ ] Forward+ (RenderingDevice) の経路で適合テストを流す手順 (lavapipe など) があり、流した結果をメモに書いた (流せない環境なら、手順と確かめていないことをメモに)
- [ ] 3 色のモードを使う BC1 の適合テストのケースがあり、Godot の出力が参照と合う
- [ ] README にある Godot の検査が通る
- [ ] `just ci` が通る

## 参考

- T0460・T0223 のメモ
- bindings/godot/README.md、ADR 0030

## メモ

- T0032 で起票した (元の候補: T0460 (2 つ)・T0223 (Godot の BC1))。
