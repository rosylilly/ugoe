---
id: T0460
title: "Godot: 描画を頂点だけの書き換え (mesh_surface_update_vertex_region) にするかを測って決める"
status: done
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
- [x] 頂点だけの書き換えを試し、効けば入れる (描画の結果が変わらないことを適合テストで確かめる)。効かなければ測った値と理由をメモに書いて入れない
- [x] `just godot-check`・`just conformance`・`just ci` が通る

## 参考

- T0450 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0450 の 2 つ)。同じ bindings/godot を触る T0459 の後に着手する。

### やったこと

- 要素とマスクのカバレッジを、読み込み時に `RenderingServer.mesh_create` + `mesh_add_surface_from_arrays` (2D の頂点・UV・頂点番号)
  で作ったメッシュにし、`canvas_item_add_mesh` を 1 度だけ足す。毎フレームは位置が変わった要素だけ、頂点のバッファを
  `mesh_surface_update_vertex_region` で書き換える (`render.rs` の `VertexMesh`)。
  - キャンバスアイテムの矩形 (画面の外の選別) はメッシュを足したときの AABB から作り直されないので、書き換えるたびに
    `canvas_item_set_custom_rect` で今の外接矩形を設定する。
  - 頂点のバッファの並び (`f32` の x, y) は Godot の内部の形式なので、作った直後に `mesh_get_surface` の `vertex_data` の大きさが
    頂点数 × 8 かを確かめ、違えば警告を出して従来の `canvas_item_add_triangle_array` で描く。
  - 裏面カリングの要素は描く三角形 (頂点番号) が変わるので、従来どおり三角形の配列を足し直す。
- README の「描画のしかた」に書いた。

### 測った値

機械は空いていなかった (load average 12〜30 / 24 コア。CI の `godot` のジョブでは流していない)。そのため受け入れ条件の 1 つめにはチェックを付けていない (今の描画の時間は下に記録した)。リリースビルド、Xvfb + `opengl3` (llvmpipe)。

`bench.gd` (`_process` の時間、2000 フレーム。一時的に環境変数で 2 つのやり方を切り替えて、交互に 2 回ずつ。単位 us、中央値):

| ケース | 三角形の配列 (今) | メッシュの頂点の書き換え |
| --- | --- | --- |
| avatar | 311 / 213 | 201 / 142 |
| avatar-lit | 311 / 211 | 187 / 238 |
| avatar-lit + scene light | 353 / 392 | 168 / 260 |
| dress-up | 171 / 232 | 75 / 78 |

端から端ではぶれが大きいので、同じ実行の中でも比べた (一時的なスクリプトで、コミットしていない):

- **同じフレームに 2 体** (片方は三角形の配列、片方はメッシュ。処理の順を入れ替えて 2 回。先に処理する方が不利なので両方の順の平均):
  `_process` の中央値が avatar 320 → 183us、avatar-lit 360 → 211us、dress-up 224 → 130us。
- **1 体を 100 フレームごとに切り替え** (60 区間、2 回): `_process` の中央値が avatar 220 → 128 / 353 → 223us、avatar-lit 304 → 203 /
  289 → 182us、dress-up 227 → 158 / 181 → 133us。描画の CPU 時間 (`viewport_get_measured_render_time_cpu`、約 1.6〜2.5ms。大半は llvmpipe)
  とフレームの間隔の差は ±数 % で、どちらにも傾かない (ぶれの内)。描画の側が遅くなる様子は無い。
- まとめ: `_process` (評価 + 描画の準備) が 1 フレームあたり約 70〜150us (約 30〜45%) 減る。効くので入れた。
- 適合テストが頂点の書き換えを確かめていることも確かめた (`mesh_surface_update_vertex_region` を呼ばないようにすると `masks-moving` が落ちる)。

### 確かめていないこと

- 実際の GPU (Forward+ / Mobile の RenderingDevice の経路、Vulkan・D3D12・Metal) での描画の側の時間と結果。測ったのも適合テストも
  Compatibility (opengl3) + llvmpipe だけ。`mesh_surface_update_vertex_region` は RD の経路ではバッファの更新になるので、
  描画が同じになることは期待できるが確かめていない。

### 検査

- `cargo test` (41 件) / `cargo clippy --all-targets -D warnings` / `cargo fmt` (bindings/godot): 通る
- `node bindings/godot/build.mjs --check --conformance` (rebase 後): ライセンス ok、smoke ok、適合テスト 63 / 63、ktx2 ok
- `just conformance` / `just ci` (rebase 後): 通る

### 起票の候補

- CI の `godot` のジョブ (か静かな機械) で `bench.gd` を流し、端から端の時間を記録する (今回は機械が空かなかった)。
- RenderingDevice の経路 (Forward+) で適合テストを流せるようにする (今は opengl3 だけ。GPU のある機械か lavapipe が要る)。
