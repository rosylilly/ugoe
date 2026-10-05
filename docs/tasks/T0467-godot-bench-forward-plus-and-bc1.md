---
id: T0467
title: "Godot: bench.gd の計測・Forward+ の経路の適合テスト・BC1 の 3 色の形のケース"
status: done
milestone: M9
crates: [bindings/godot, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0467
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
- [x] Forward+ (RenderingDevice) の経路で適合テストを流す手順 (lavapipe など) があり、流した結果をメモに書いた (流せない環境なら、手順と確かめていないことをメモに)
- [x] 3 色のモードを使う BC1 の適合テストのケースがあり、Godot の出力が参照と合う
- [x] README にある Godot の検査が通る
- [x] `just ci` が通る

## 参考

- T0460・T0223 のメモ
- bindings/godot/README.md、ADR 0030

## メモ

- T0032 で起票した (元の候補: T0460 (2 つ)・T0223 (Godot の BC1))。

### やったこと (2026-10-05、claude-worker-T0467)

**BC1 の 3 色のモード**

- `ugoe_headless::image::Ktx2Compression::Bc1ThreeColor` を足した (`bc.rs`。すべてのブロックを `color0 <= color1` で書き、黒い画素は添字 3)。
  単体テスト: 往復の誤差・すべてのブロックが 3 色のモード・添字 3 の数・不透明な黒に読むこと。
- 適合テスト: 生成するモデル `bc1-three-color` (`nested-masks` のテクスチャを不透明にし、斜めの黒い線 `(x + y) % 4 == 0` を引いて
  `BC1_RGB_UNORM_BLOCK` の 3 色のモードにしたもの) とケース `ktx2-bc1-three-color` を足した。
- **Godot は参照と違った** (直す前: 画像 92.0% 一致、最大 131/255。黒い線が透けて背景の灰色になった)。Godot は BC1 を
  `FORMAT_DXT1` (1 ビットのアルファ付き) として `Image.decompress` で展開し、4 つ目の色を透明にするため。
- 扱いの判断 (仕様の判断は要らなかった): KTX2 の `vkFormat` は Vulkan の形式で、`VK_FORMAT_BC1_RGB_*_BLOCK` は「アルファが無く不透明」
  (Vulkan の仕様と Khronos Data Format Specification の S3TC の節: アルファの無い BC1 の 3 色のモードの 4 つ目は黒で、アルファは 1)。
  Direct3D には BC1 のアルファ無しの形式が無く (`DXGI_FORMAT_BC1_UNORM` は常に 1 ビットのアルファ付きで 4 つ目は透明)、Godot はその規則で
  読んでいる。仕様 (`docs/spec/rendering.md#前提`・英語の `spec/rendering.md`) はすでに「`BC1_RGB` はアルファを 255」と決めていて参照も
  そうなので、参照が正しい。Godot の側で、`BC1_RGB_*` の KTX2 のテクスチャは展開した後にアルファを 255 にした
  (`src/model.rs` の `ktx2_is_bc1_rgb` と `render.rs` の `load_texture`。単体テストを足した)。法線マップは RGB だけを読むので変えていない。
- 直した後: Godot (Compatibility・Forward+)・c・wasm・wasm-webgpu・unity でこのケースが通る。
- README (bindings/godot・tests/conformance) と `docs/spec/rendering.md#前提` (Godot の扱い) に書いた。

**Forward+ (RenderingDevice) の経路**

- `build.mjs --conformance --renderer forward_plus` (= `run.mjs --only godot --godot-renderer forward_plus`) で、
  `--rendering-method forward_plus --rendering-driver vulkan` で描く。GPU が無ければ Mesa の lavapipe (`VK_DRIVER_FILES` で lavapipe の ICD を指す)。
- Godot は Vulkan を作れないと黙って Compatibility に戻る (`switching to OpenGL 3`) ので、`conformance.gd` に描き方を渡し、違えば止まるようにした
  (ICD を存在しないファイルにして、止まることを確かめた)。`bench.gd` も同じ。
- **結果 (この機械、WSL2・Mesa 26.2.3 の lavapipe・Godot 4.7.1):** `Vulkan 1.4.354 - Forward+ - llvmpipe` で 67 / 67 ケースが通った
  (画像の最悪は `ktx2-bc-avatar-lit` の 3/255、ほかは 0〜1/255)。T0460 の `mesh_surface_update_vertex_region` (RD の経路ではバッファの更新)
  も `masks-moving`・`avatar-moving` で確かめたことになる。
- CI の `godot` のジョブに、`mesa-vulkan-drivers` と lavapipe への限定を足し、`just godot-check` の後に Forward+ でも適合テストを流すようにした。
  落ちたら `target/conformance` を成果物 `godot-conformance` に残す。

**bench.gd**

- `build.mjs --release --bench [--renderer forward_plus]` を足した。`bench.gd` の最初の行に Godot の版・描き方・アダプタ・論理プロセッサの数を出す。
- CI の `godot` のジョブの最後で、リリースビルドの bench.gd を Compatibility と Forward+ で流し、ジョブの要約 (`$GITHUB_STEP_SUMMARY`) と
  成果物 `godot-bench` に残す (`continue-on-error`。測るだけで検査は落とさない)。
- 受け入れ条件の 1 つめ (時間を README か docs に記録) は、CI で流れた後に回す (オーケストレーターの指示。チェックしていない)。
  参考までにこの機械 (load average 10〜26 / 24 コアで静かではない) で流した値 (`_process`、2000 フレーム、us):

  | ケース | Compatibility (mean / median / p95) | Forward+ lavapipe (mean / median / p95) |
  | --- | --- | --- |
  | avatar | 230 / 216 / 349 | 189 / 174 / 296 |
  | avatar-lit | 269 / 242 / 440 | 165 / 157 / 245 |
  | avatar-lit + scene light | 296 / 275 / 464 | 387 / 335 / 707 |
  | dress-up | 176 / 166 / 275 | 147 / 135 / 227 |

### 範囲外の変更 (小さなもの)

- `crates/ugoe-headless` (`bc.rs`・`image.rs`): `Ktx2Compression::Bc1ThreeColor` (公開の列挙に 1 つ足した。テスト用の書き出し)。
- `.github/workflows/ci.yml` の `godot` のジョブ (lavapipe・Forward+ の適合テスト・bench.gd・成果物)。

### 確かめていないこと

- CI のランナー (ubuntu-24.04 の Mesa 24 の lavapipe) で Forward+ が動くか (手元は Mesa 26.2.3)。動かなければ実行器が描き方の食い違いで止まる。
  CI で流れた後に確かめる。bench.gd の CI の値も同じく、流れた後に README に記録する。
- 実際の GPU (Vulkan・D3D12・Metal) と Mobile の描き方。
- Godot が `Image.decompress` に失敗した (展開できない) ときの BC1 の 4 つ目の色 (警告を出して GPU のまま描く経路。今の Godot 4.7 では BCn は常に展開できる)。

### 起票の候補

- CI の `godot` のジョブの bench.gd の結果を bindings/godot の README に記録する (受け入れ条件の 1 つめ。CI で流れた後)。
- CI で Forward+ が lavapipe で動かなかったときの対処 (Mesa の版を上げる PPA など)。

### 検査

- `git rebase main` の後に: `just ci` が通った。`just godot-check` (ライセンス・smoke・適合テスト 67 / 67)、Forward+ (lavapipe) の適合テスト 67 / 67、
  `just conformance --only c,wasm,wasm-webgpu,unity --cases ktx2-bc1-three-color,ktx2-bc-avatar-lit` が通った。
  bindings/godot の `cargo test` (42 件)・`cargo clippy --all-targets -D warnings`・`cargo fmt` も通った。
