---
id: T0450
title: "Godot: 毎フレームの確保を減らし、マスクのビューポートの作り損ねを後始末する"
status: done
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

- [x] 1 フレームの時間を測り、効くものだけを直した (測った値をメモに書く)
- [x] 作り損ねたビューポートを解放する
- [x] `node bindings/godot/build.mjs --check` と `--conformance` が通る
- [x] `just ci` が通る

## 参考

- bindings/godot/src/render.rs、bindings/godot/src/node.rs

## メモ

### 測り方

- `example/tests/bench.gd` を足した (検査ではない。README の「検査」に書いた)。見本 (`avatar`・`avatar-lit`・`avatar-lit` +
  動く `PointLight2D` (`use_scene_lights`)・`dress-up`) のすべてのパラメータを毎フレーム揺らし、`UgoePuppet2D` の `_process` の前後を
  `process_priority` の違うノードで挟んで 2000 フレーム測る。リリースビルド、Xvfb + `opengl3`。
- 機械が並行のビルドで重く (load average 約 39 / 24 コア)、端から端の時間はぶれが大きすぎて差を見分けられなかった
  (同じライブラリで 1 フレームの中央値が 280〜540us)。そこで、一時的に計時を `render.rs`・`node.rs` に入れて (コミットしていない)、
  同じ実行の中で区間ごとの時間と、2 つのやり方を同じ要素で続けて呼んだ時間を比べた。

### 測った値 (変える前。1 フレームあたり)

| 区間 | avatar (24 要素) | 備考 |
| --- | --- | --- |
| `_process` 全体 | 約 300〜450us | 目標の 1ms の内 |
| `Puppet::update` | 約 55〜90us | |
| `Renderer::draw` | 約 230〜420us | 大半は `canvas_item_add_triangle_array` (RenderingServer 側の処理) |
| `draw` の中の `PackedVector2Array` を作る (+ カリングの `PackedInt32Array`) | 約 6〜15us (draw の約 3%) | |
| `draw` の中の `set_color_params` | 約 30〜55us (draw の約 14%) | |
| シーンの光源: 探し直し (`find_scene_lights`) | 約 1us | |
| シーンの光源: 値を作って比べる (`scene_light_values`・`manual_lights` の複製・`LightUniforms::new`) | 約 6〜20us | ほぼ Godot の呼び出し (光源 1 つにつき約 10 回)。`Vec` の確保は 0.1us 程度 |
| 光源が変わったときの `set_lighting` | 約 20〜80us | 光源が毎フレーム動くときだけ |

同じ要素で 2 つのやり方を続けて呼んだ比較 (順番を入れ替えて 2 回。先に呼んだ方が不利になるので、両方の順の平均で見た):

- 頂点の配列: 作り直す (`collect`) 約 290ns / 使い回して書く (`resize` + `as_mut_slice`) 約 165ns。1 要素あたり約 0.13us、24 要素で約 3us (1 フレームの約 1%)
- `set_shader_parameter` の名前: `&str` (呼ぶたびに `StringName` を作る) と作っておいた `StringName` の差が 1 回あたり約 0.2us
  (2 回目以降の呼び出しで `&str` 約 300ns / `StringName` 約 125ns)。位置が変わる要素は 4 つの uniform を設定するので、24 要素で約 20us (約 5%)

### 直したもの

- **uniform の名前を `StringName` で持つ** (`UniformNames`。毎フレームの色の 4 つと光源の 4 つ)。`set_color_params` の時間が約 14% → 約 11% (draw に対して)。
- **光源の uniform の値を 1 回だけ作る** (`LightParams`)。以前は要素ごとに `PackedVector4Array` を 2 つ作り直していた。
  光源が毎フレーム動くときの `set_lighting` が約 56〜80us → 約 35〜48us (同じ条件の別の実行で比べた)。
- **作り損ねたビューポートを解放する。** `get_texture()` を、根と前の段を子にする前に確かめ、無ければ `free()` してやめる。
  以前は、作り損ねた段のビューポートが木に入らずに残るうえ、前の段のビューポートがその子になっていたので、その後の
  `owner.add_child(前の段)` も親が既にあって失敗していた。Godot では起きない経路なので、テストでは再現できない (確かめたのは、
  ふつうの経路が smoke と適合テストで変わらないことだけ)。

### 直さなかったもの (効かない)

- 頂点の `PackedVector2Array` とカリングの `PackedInt32Array` の使い回し: 約 3us / フレーム (約 1%)。一度入れて測り、効きが小さいので戻した。
- マスクの `moved` の `Vec`: マスクの数だけの `bool` の確保 1 回で、測れる差は無い。
- `use_scene_lights` の `scene_light_values` の `Vec`・`manual_lights` の複製・`LightUniforms`: 確保は 0.1us 程度で、時間は Godot の呼び出しが占める。

### 検査

- `cargo test` (36 件) / `cargo clippy --all-targets -D warnings` / `cargo fmt` (bindings/godot): 通る
- `node bindings/godot/build.mjs --check --conformance` (rebase 後): ライセンス ok、smoke ok、適合テスト 57 / 57、ktx2 ok
- `just ci` (rebase 後): 通る

### 起票の候補

- `Renderer::draw` の大半は `canvas_item_add_triangle_array` (毎フレーム、位置が変わった要素ごとに RenderingServer がポリゴンを作り直す)。
  減らすなら、`canvas_item_add_triangle_array` の代わりに `RenderingServer.mesh_*` で頂点だけを `mesh_surface_update_vertex_region` で
  書き換える方法がある (UV と頂点番号は変わらないので)。効くかどうかは測ってから。
- 端から端の時間を比べるには、静かな機械か CI (`godot` のジョブ) で `bench.gd` を流すとよい。
