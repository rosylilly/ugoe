---
id: T0134
title: ランタイム: 受け手でも物理より後の段 (リップシンクなど) を同じ順に当てる
status: todo
milestone: M8
crates: [ugoe, ugoe-ffi, ugoe-wasm, bindings/web, tests/conformance, docs]
depends_on: [T0133]
assignee:
---

# T0134: 受け手の段の順をそろえる

## 目的

T0133 の `follow` は、指揮役が送った値 (リップシンクを済ませた後の値) を基準値にして物理を回す。1 つの Puppet では物理がリップシンクの前の値を読むので、
リップシンクの対象を物理の入力にするモデルでは、受け手の揺れが 1 つの Puppet と一致しない (T0133 のメモ・docs/spec/motion.md の制限)。
スタジオはマイクの口パク (T0803) を指揮役で当てるので、ADR 0028 の「受け手は物理まで同じ形」を守るために順序をそろえる。

## 受け入れ条件

- [ ] 案 C (T0133 の担当の提案を team-lead が採用): フレームを {dt, values, lip_sync} にする。values は物理の段に入る直前の値 (Puppet に `values_before_physics()` を足す)、
      lip_sync は音量と強さの 2 つの f32。受け手の `follow(values, lip_sync, dt)` は、基準値の書き込み → 物理 → 自分のモデルの設定でリップシンク → ポーズ → 評価の順に進める
      (Puppet::update と同じ段の順)。既存の `update` と `set_base_values` の挙動は変えない
- [ ] リップシンクの対象を物理の入力にするモデルで、指揮役と受け手の組み合わせが 1 つの Puppet と許容誤差内で一致するテスト (適合テストのケースも足す)
- [ ] docs/spec/motion.md の制限の記述を消し、docs/studio.md のフレームの形を更新する
- [ ] `just ci`・`just wasm-check`・`just ffi-check`・`just conformance --only c,wasm` が通る

## 参考

- docs/tasks/T0133-runtime-explicit-base-values.md のメモ、docs/adr/0028-studio.md

## メモ
