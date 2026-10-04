---
id: T0460
title: "Godot: 描画を頂点だけの書き換え (mesh_surface_update_vertex_region) にするかを測って決める"
status: in-progress
milestone: M9
crates: [bindings/godot, docs]
depends_on: [T0459]
assignee: claude-worker-T0460
---

# T0460: Godot: 描画を頂点だけの書き換え (mesh_surface_update_vertex_region) にするかを測って決める

## 目的

T0450 で毎フレームの確保を減らしたが、`Renderer::draw` (`bindings/godot/src/render.rs`) の大半は `canvas_item_add_triangle_array` で、
位置が変わった要素ごとに RenderingServer がポリゴンを作り直す。UV と頂点番号は変わらないので、`RenderingServer.mesh_*` で作ったメッシュの
頂点だけを `mesh_surface_update_vertex_region` で書き換える方法がある。効くかは測ってから決める (T0450 の候補)。

## 受け入れ条件

- [ ] 静かな機械か CI の `godot` のジョブで `bench.gd` を流し、今の描画の時間を記録した
- [ ] 頂点だけの書き換えを試し、効けば入れる (描画の結果が変わらないことを適合テストで確かめる)。効かなければ測った値と理由をメモに書いて入れない
- [ ] `just godot-check`・`just conformance`・`just ci` が通る

## 参考

- T0450 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0450 の 2 つ)。同じ bindings/godot を触る T0459 の後に着手する。
