---
id: T0313
title: 物理の出力の加算モード
status: todo
milestone: M3.5
crates: [ugoe-model, ugoe-physics, ugoe-editor-core, apps/editor, docs]
depends_on: [T0312]
assignee:
---

# T0313: 物理の出力の加算モード

## 目的

Live2D の実践では、トラッキングの値と物理の揺れを両立させるため、体のパラメータを「トラッキング用」と「物理用」に複製し、両方のキーを付けている (参考の記事 4)。
ugoe の物理のパラメータへの出力は、今の値を物理の値へ置き換える (`param + (value - param) * weight`) ので、同じ回避策が要る。
出力に「加算」を選べるようにし、トラッキングの値 (や手で動かした値) に物理の揺れを足せるようにする。

## 受け入れ条件

- [ ] `PhysicsOutput` に合成の方法 (`overwrite` (既定、今と同じ) / `add`) を足す。`add` は物理を進める前の値 (フレーム内の順序で保存した基準値) に `(value - 静止時の値) * weight` を足し、値域にクランプする。フレームごとに累積しないこと。古いデータは `overwrite` として読む
- [ ] トラッキングの `AngleX` から、同じ `BodyAngleX` に「トラッキング × 係数 + 物理の遅れ」を出すテスト (パラメータを複製しない)
- [ ] エディタの物理のパネルで出力ごとに選べる
- [ ] `docs/spec/physics.md`・`docs/spec/model.md`・`docs/spec/file-formats.md` (未知の値の扱い)。`just ci`・`just gen`・`just wasm-check` が通る

## 参考

- https://note.com/himono_vtuber/n/n9015f0030d53
- `docs/spec/motion.md#フレーム内の適用順`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
