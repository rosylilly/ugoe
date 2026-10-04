---
id: T0699
title: ビューポートのツールの操作をコンポーネントのテストで確かめる
status: in-progress
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0672]
assignee: claude-worker-T0699
---

# T0699: ビューポートのツールの操作をコンポーネントのテストで確かめる

## 目的

ビューポートのツール (`apps/editor/src/lib/viewport/tools/` の `DeformerGizmo`・`FormTool`・`MeshEditTool`・`PhysicsOverlay`・
`LightingOverlay`・`GlueOverlay`) は、幾何の純粋な関数 (`geometry.ts`・`gizmo.ts`・`form.ts`・`physicsGeometry.ts`) だけに
テストがあり、ポインタの押す・動かす・離す・取り消しから送るコマンド (`set_keyform_cell`・`move_mesh_vertices`・
`set_physics_collider` など) までの流れはテストが無い。T0672 で直した誤り (取り消しでコマンドを送る・送っている間の
押下で表示がずれたまま残る・回転が ±180 度で折り返す) は、どれもこの流れの中にあった。

エディタのテストはノードの環境で動いている (`apps/editor/vite.config.ts`)。`packages/ui` のブラウザでのテスト
(`overlays.test.ts`) と同じ仕組みか、ノードの DOM の実装でツールを描き、偽の `ViewportController`・`EditorDocument` で
コマンドの列を確かめる。

## 受け入れ条件

- [ ] ツールのコンポーネントを描いてポインタの操作を送れるテストの仕組みを決め、docs/testing.md に書く
- [ ] ギズモ (ワープの点・回転・拡大率・原点)・フォーム編集・メッシュ編集・衝突体のドラッグについて、送るコマンドと取り消しを確かめる
- [ ] ダイアログの中のキー操作 (Esc・Delete・Ctrl (Cmd) + A) がツールに届かないことを確かめる
- [ ] `just ci` が通る

## 参考

- T0672 のメモ (所見の表)
- docs/testing.md

## メモ
