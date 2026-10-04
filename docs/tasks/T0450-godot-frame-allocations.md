---
id: T0450
title: "Godot: 毎フレームの確保を減らし、マスクのビューポートの作り損ねを後始末する"
status: in-progress
milestone: M9
crates: [bindings/godot]
depends_on: [T0427]
assignee: claude-worker-T0450
---

# T0450: Godot: 毎フレームの確保を減らし、マスクのビューポートの作り損ねを後始末する

## 目的

T0427 のレビューで見つけた、小さいが測ってから直したい点をまとめる。

- `Renderer::draw` は、頂点が変わった要素ごとに `PackedVector2Array` を作り直し (`self.points = ...collect()`)、カリングする要素では
  `PackedInt32Array` も毎回作る。マスクがあれば `moved` の `Vec` も毎フレーム確保する。
- `use_scene_lights` のときは、毎フレーム `scene_light_values` の `Vec` と `manual_lights` の複製と `LightUniforms` を作る
  (変わっていなければ捨てる)。
- `Renderer::new` で、段のビューポートの `get_texture()` が `None` だと、その段のビューポート (と根の `Node2D`) を木に入れずに
  `break` するので解放されない (Godot では起きないはずの経路)。

RenderingServer に渡した配列は Godot が参照で持つ (コピーオンライト) ので、使い回しても書き込みで複製が起きうる。
見本のモデル (`assets/samples/`) で 1 フレームの時間を測り、効くものだけ直す (ランタイムの目標は 1 体 1ms。roadmap.md の M7)。

## 受け入れ条件

- [ ] 1 フレームの時間を測り、効くものだけを直した (測った値をメモに書く)
- [ ] 作り損ねたビューポートを解放する
- [ ] `node bindings/godot/build.mjs --check` と `--conformance` が通る
- [ ] `just ci` が通る

## 参考

- bindings/godot/src/render.rs、bindings/godot/src/node.rs

## メモ

