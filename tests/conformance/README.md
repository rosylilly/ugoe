# 配布物の適合テスト (`ugoe-conformance`)

**層:** ツール層 (テスト用。依存してよいクレートは `scripts/check-layers.mjs`)

すべての配布物で、同じモデル・同じパラメータから同じ頂点と画像が得られることを検査する
([ADR 0017](../../docs/adr/0017-single-runtime-distribution.md) の決定 3、[docs/portability.md](../../docs/portability.md#配布))。
参照は `ugoe::Puppet` (の `ModelInstance`) と `ugoe render` で作り、各バインディングの出力を公開仕様の許容誤差で比べる。

| 配布物 (`--only` の名前) | 頂点 | 画像 | 実行器 |
| --- | --- | --- | --- |
| C ABI (`ugoe-ffi`、`c`) | 位置・不透明度・描画順の値・乗算色・スクリーン色・マスクの効き具合・`visible`・描画の順番 | なし (C ABI は描画しない) | [`c/runner.c`](c/runner.c) |
| wasm (npm パッケージ `@ugoe/runtime`、`wasm` と `wasm-webgpu`) | C ABI と同じすべての値 (`drawable_positions`・`drawable_states_json`・`render_order`) | あり (ヘッドレスの Chrome の WebGL2 と WebGPU。`read_pixels` で読み戻す) | [`web/runner.js`](web/runner.js) |
| `<ugoe-avatar>` (npm パッケージ `@ugoe/runtime` の要素、`avatar`) | 参照とは比べない。シナリオ (下の「`<ugoe-avatar>` のシナリオ」) の成否を見る | 要素の canvas に描いたか (背景でない画素があるか)。ヘッドレスの Chrome の WebGL2 | [`web/avatar.js`](web/avatar.js) |
| 問い合わせの JSON (npm パッケージ `@ugoe/runtime` の `*_json()`、`queries`) | 参照とは比べない。出力のキーを、パッケージの型のキーの一覧と比べる (下の「問い合わせの JSON のキー」) | なし (ヘッドレスの Chrome の WebGL2 で `WebPuppet` を作るだけ) | [`web/queries.js`](web/queries.js) |
| 見本の MediaPipe の利用状況の送信 (`bindings/web/examples/basic`、`example`) | 参照とは比べない。「camera (MediaPipe)」で Face Landmarker を作った後に、送り先 (`odml.pa.googleapis.com`) への要求がネットワークに出ないか (下の「見本の MediaPipe の利用状況の送信」) | なし (ヘッドレスの Chrome の WebGL2 と偽のカメラ) | [`example.mjs`](example.mjs) |
| Unity (`bindings/unity` の Unity に依存しない C# の層 `Runtime/Core`、`unity`) | C ABI と同じすべての値。テクスチャを持つバンドルのケースはテクスチャのデコードの結果 (`textures`) も | なし (描画は Unity が要る) | [`unity/Program.cs`](unity/Program.cs) (dotnet) |
| Godot (`bindings/godot`、`godot`) | C ABI と同じすべての値 (ノードの `get_drawable_positions`・`get_drawable_state`・`get_render_order` で読み戻す) | あり (Compatibility (OpenGL)。`--godot-renderer forward_plus` で Forward+ (RenderingDevice の Vulkan。GPU が無ければ lavapipe)。画面が無ければ Xvfb) | [`bindings/godot/example/tests/conformance.gd`](../../bindings/godot/example/tests/conformance.gd) |

## 実行

```sh
just conformance                          # = node tests/conformance/run.mjs (すべてのバインディング)
just conformance --only c                 # C ABI だけ (Chrome も GPU も要らない)
just conformance --only wasm,wasm-webgpu  # wasm だけ (WebGL2 と WebGPU)
just conformance --only avatar            # <ugoe-avatar> のシナリオだけ (参照を作らないので cargo のビルドも要らない)
just conformance --only queries           # 問い合わせの JSON のキーだけ (参照を作らないので cargo のビルドも要らない)
just conformance --only example           # 見本の MediaPipe の利用状況の送信だけ (pnpm install 済みの packages/face-tracking と、モデルの取得が要る)
just conformance --only unity             # Unity の C# の層だけ (dotnet と C ABI のライブラリ)
just conformance --only godot             # Godot だけ (= just godot-check の比べ合わせ)
just conformance --only godot --godot-renderer forward_plus   # Godot を Forward+ (Vulkan) で描く (既定は compatibility = opengl3)
just conformance --cases avatar-pose,masks-moving   # ケースを絞る
just conformance --no-web-build           # bindings/web/dist を作り直さない (--no-godot-build は Godot の拡張)
```

結果は `target/conformance/` (`--out` で変えられる) に残る。

```
target/conformance/
  plan.json  plan.txt          計画 (展開したケース)
  models/                      計画の `generated` のモデル (src/bundles.rs が作る)
  reference/<ケース>.json .png  参照
  c/<ケース>.json               C ABI の結果
  wasm/<ケース>.json .png       wasm (WebGL2) の結果。画像が合わなければ <ケース>.diff.png (違う画素が赤)。chrome.log (ページのコンソールを含む)
  wasm-webgpu/                 wasm (WebGPU) の結果 (同じ形)
  avatar/avatar.json           <ugoe-avatar> のシナリオごとの結果 ("ok" か失敗の理由)。chrome.log
  queries/queries.json         問い合わせの JSON のキーの項目ごとの結果 ("ok" か食い違いの一覧) と、形ごとに比べた数。chrome.log
  example/example.json         見本を開いた 2 回 (guarded・unguarded) の結果と、記録した送り先への要求。chrome-<guarded|unguarded>.log。
  unity/<ケース>.json           Unity の C# の層の結果
  godot/<ケース>.json .png      Godot の結果。skipped.json (扱わなかったケースと理由。今は空)
```

要るもの (`just ci` に含めない理由): C コンパイラ (`cc`。`CC` で変えられる)、Chrome (`CHROME` で指定。無ければ
`google-chrome`・`chromium` などを探す)、wasm-bindgen-cli (`just wasm-bindgen-version` の版。パッケージのビルド)、
参照の画像を描く GPU (無ければ Mesa の lavapipe / llvmpipe)、.NET SDK 9 以降 (`unity`)、Godot 4.4 以降 (`GODOT` で指定。既定は `godot`) と、
画面が無ければ xvfb-run (`godot`)。

**足りない道具の扱い (T0418 で決めた):** `run.mjs` は最初に道具を調べる。

- `--only` を付けないとき: `c`・`wasm`・`wasm-webgpu` の道具が足りなければ、足りないものと入れ方を出して止まる (終了コード 2)。
  `unity` (dotnet) と `godot` (Godot・xvfb-run) は、足りなければ入れ方を出して **そのバインディングを飛ばし**、最後に `skipped` と表示する。
  エンジンの道具は重く、C ABI と wasm だけを確かめたい人や CI のジョブが多いため。
- `--only` で名前を挙げたとき: 挙げたバインディングの道具が足りなければ、どれでも止まる (飛ばしたのに通ったように見えないように)。

CI では `.github/workflows/ci.yml` の `conformance` ジョブが `just conformance` を実行し (`<ugoe-avatar>` のシナリオ・問い合わせの JSON のキー・見本の MediaPipe の利用状況の送信も含む。ランナーに dotnet があれば Unity も、Godot は無いので飛ぶ)、
`godot` ジョブの `just godot-check` (`build.mjs --conformance`) が `--only godot` を、続けて `--renderer forward_plus` で
Forward+ (lavapipe) でも実行する (T0467)。落ちたら `target/conformance` を成果物として残す。
このクレートの単体テスト (ケースの展開・生成するモデル・比べ合わせ・参照の評価) は `cargo test` に入るので `just ci` で動く。

## 流れ

1. `ugoe-conformance reference --out DIR [--only ケース,...]`: 計画の `generated` のモデルを `DIR/models/` に作り
   ([`src/bundles.rs`](src/bundles.rs))、[`cases.json`](cases.json) を計画に展開し (`DIR/plan.json`・`DIR/plan.txt`)、
   ケースごとに参照の評価の結果を `DIR/reference/<ケース>.json` に書く ([`src/reference.rs`](src/reference.rs))。
2. 画像を比べるバインディングがあれば、`run.mjs` がケースごとに `ugoe render <plan.json の render_args> -o DIR/reference/<ケース>.png` を実行する。
3. 各バインディングの実行器が、計画を読んで `DIR/<バインディング>/<ケース>.json` (と `.png`) を書く。扱えないケースは
   `DIR/<バインディング>/skipped.json` (`{"<ケース>": "理由"}`) に書いてよい (比べ合わせは `skip` と表示し、失敗に数えない)。
4. `ugoe-conformance compare --out DIR --binding <名前> [--images | --images-only]` が比べる ([`src/compare.rs`](src/compare.rs))。
   `--images-only` は評価の結果を読み戻せないバインディング向けで (今は使うものが無い。Godot は T0420 で読み戻すようにした)、
   画像の無いケースは飛ばす。1 つも比べなければ失敗。

### ケース (`cases.json`)

| フィールド | 内容 |
| --- | --- |
| `name` / `summary` | 名前 (出力のファイル名。英数字・`-`・`_`) と説明 |
| `model` | `.ugoe` かモデルの JSON (リポジトリのルートからの相対パス) |
| `generated` | `model` の代わりに、[`src/bundles.rs`](src/bundles.rs) が作るモデルの名前 (`nested-masks`・`mixed-masks`・`intersection-masks`・`blend-modes`・`many-masks`・`moving-masks`・`lip-sync-hair`・`part-switch`・`avatar-ktx2`・`avatar-lit-ktx2`・`ktx2-mips`・`avatar-bc3`・`avatar-lit-bc`・`bc1-three-color`) |
| `spec` | `spec/examples/<例>` を書くと、その `expected.json` のケースごとに展開する (`spec-<例>-<番号>`。モデルとパラメータはそこから取る) |
| `modules` | 付ける `.ugoemod` (付ける順) |
| `parameters` | パラメータの値 (値域の外はクランプされる) |
| `before` | 先に描く値 (ID は `parameters` にもあること)。下の「実行器の約束」の 3 |
| `time` / `fps` | 時間を進める秒数と 1 秒あたりの `update` の回数 (既定 60)。`ugoe render --time --fps` と同じ |
| `effects` | 時間を進めるときに目パチと呼吸を使うか (既定 `false`) |
| `follow` | 受け手として進めるか (既定 `false`。T0133)。`time` と `"image": false` が要る。計画は、物理を切った指揮役の `Puppet` (`effects` なら標準の目パチと呼吸を使う) を `time` の列で進めた各フレームの値を `follow` に持ち、実行器は `update` の代わりに `follow(値, dt)` を呼ぶ (下の「実行器の約束」の 3) |
| `lip_sync` | `follow` のケースで、指揮役がモデルのリップシンクの設定にフレームごとに変わる音量を当てるか (既定 `false`。T0134)。計画の `follow_lip_sync` に各フレームの `[音量, 強さ]` が入る |
| `motion` | `follow` のケースで、指揮役が最初に再生するバンドルのモーションの名前 (T0135)。パーツの不透明度のカーブがあれば、計画の `follow_part_opacities` に各フレームの `[パーツの番号, 値, 重み]` の並びが入る |
| `motions` | 時間を進める途中で始めるバンドルのモーション `[{"step", "name", "priority", "accepted"}]` (`step` の順。T0462)。`time`・バンドル・`"image": false` が要り、`follow` とは一緒に使えない。下の「実行器の約束」の 3 |
| `lighting` | 光源 (画像だけに効く)。形は `WebPuppet::set_lighting_json` と同じ |
| `image` | 画像も比べるか (既定 `true`) |

入力は見本 (`assets/samples/avatar.ugoe`・`avatar-lit.ugoe`・`dress-up/`)、生成するマスクと合成モードのモデル (もとは Godot の
`build.mjs --conformance` だけにあったもの。T0418 で移した)、公開仕様の評価の例 (`spec/examples/*/expected.json`) と
バンドルの例 (`spec/examples/bundles/`: zstd の圧縮・任意の機能・法線マップ・物理)、KTX2 のテクスチャ (T0222。下の段落)。描画の例 (`render.json`) は、テクスチャを
その場で作る必要があるので使っていない (参照レンダラは `crates/ugoe-render/tests/spec_render.rs` で確かめている)。

**KTX2 のテクスチャ (`ktx2-*` のケース、T0222・T0223):** 生成するモデルで、テクスチャを KTX2 にしたもの。KTX2 も乗算していない
アルファとして読んで乗算し、値は形式の `_SRGB` や転送関数によらず保存されたまま使い、ミップマップは最初の段だけを使う
([docs/spec/rendering.md#前提](../../docs/spec/rendering.md#前提))。どの実装もそう読めば、PNG のバンドルと同じ画像になる
(`src/bundles.rs` の単体テストが、デコードした画素が元の PNG と同じことを確かめる)。

- `avatar-ktx2` / `avatar-lit-ktx2`: 見本のテクスチャと法線マップを `VK_FORMAT_R8G8B8A8_SRGB` にしたもの。
- `ktx2-mips`: `nested-masks` のテクスチャを、zstd で超圧縮したミップマップ付きの `VK_FORMAT_R8G8B8A8_UNORM` にしたもの
  (2 段目からは不透明なマゼンタ)。
- `avatar-bc3` (ケース `ktx2-bc3-avatar`): 見本のテクスチャをブロック圧縮の `VK_FORMAT_BC3_UNORM_BLOCK` にしたもの
  (もとは Godot の `example/tests/ktx2.gd`。T0223 で移した)。
- `avatar-lit-bc` (ケース `ktx2-bc-avatar-lit`): ライティングの見本のテクスチャを zstd で超圧縮した `VK_FORMAT_BC7_SRGB_BLOCK` に、
  法線マップを `VK_FORMAT_BC1_RGB_UNORM_BLOCK` にしたもの。
- `bc1-three-color` (ケース `ktx2-bc1-three-color`。T0467): `nested-masks` のテクスチャを不透明にして斜めの黒い線を引き、
  すべてのブロックを BC1 の 3 色のモード (`color0 <= color1`) で書いた `VK_FORMAT_BC1_RGB_UNORM_BLOCK` にしたもの。黒は 4 つ目の色
  (添字 3) になる。アルファの無い BC1 ではこの色も不透明 (Khronos Data Format Specification の S3TC の節) で、アルファのある BC1
  (Direct3D の `DXGI_FORMAT_BC1_UNORM` や Godot の `Image.FORMAT_DXT1`) として読むと透明になって背景が透ける。

ブロック圧縮のケースは、画素が圧縮で変わるので PNG のバンドルと同じ画像にはならない (参照も同じ KTX2 を展開して描く)。
参照 (`ugoe render`) と wasm は BC1〜BC5・BC7 を `bcdec_rs` で展開する ([ADR 0030](../../docs/adr/0030-compressed-ktx2-decoding.md))。

8 ビットの RGBA ではない PNG のケース (T0473。`src/bundles.rs`):

- `png-16bit`: `masks-nested` のテクスチャを 16 ビットの RGBA にしたもの (上位の 8 ビットが元の画素、下位は位置で変わる値)。
  ガンマ 1.0 の `gAMA` を持つ。参照は上位の 8 ビットを使い、ガンマや色のチャンクを見ない ([docs/spec/rendering.md#前提](../../docs/spec/rendering.md#前提)) ので、
  `masks-nested` と同じ画素になる (ガンマを補正するデコーダでは大きく変わる。Godot の `Image.load_png_from_buffer` では 87.5% の画素しか
  一致しなかったので、Godot も参照と同じデコーダで PNG を読む。T0477)。
- `png-palette`: 16 色のパレット (`PLTE`) と色ごとのアルファ (`tRNS`) の PNG。

KTX2 は `ugoe_headless::image::encode_ktx2` で書く (乗算済みの印 `KHR_DF_FLAG_ALPHA_PREMULTIPLIED` を立てない。ブロック圧縮は
`Ktx2Compression` の簡単な圧縮で、画質は求めない)。ETC2・ASTC・Basis Universal は参照と wasm がデコードしないので、ここでは比べない。
C ABI と Unity の C# の層は描画しないので、頂点を比べる。Unity は PNG も KTX2 も C ABI のデコーダ (`ugoe_image_decode`) で読むので
(KTX2 は T0466、PNG は T0473)、テクスチャを持つバンドルのケースでは、参照と Unity の実行器がテクスチャと法線マップを番号ごとにデコードし
(描画するテクスチャは乗算済み、法線マップはアルファを掛けない)、`"<幅>x<高さ>:<画素の FNV-1a 64>"` (読めなければ失敗の種類) を結果の `textures` に書いて、
完全に一致することを確かめる (`src/frame.rs` の `TextureFrame`。書かないバインディングとは比べない)。

### 実行器の約束 (計画)

`plan.json` の各ケース (`src/plan.rs` の `Case`) と `plan.txt` (C 向けの行の形式。`Plan::to_text`) は同じ内容を持つ。実行器は次のとおりにする。

1. モデルを読む (`bundle` なら `.ugoe`、違えば JSON)。`modules` を順に付ける。
2. `parameters` を設定する。
3. `steps` が空なら、すべての段 (物理・目パチ・呼吸・リップシンク・ポーズ) を止めて 1 回評価する (時間を進めない評価。
   参照は `ModelInstance::update` 1 回で、`ugoe render` の `--time` なしと同じ)。
   `steps` があれば、目パチと呼吸を `effects` のとおりにし、ほかの段は動かして、`steps` の `dt` で順に `update` する
   (`ugoe render --time` と同じ列。実行器では `dt` を計算しない)。
   計画に `follow` (各フレームの、指揮役の物理の段に入る直前のパラメータの値。モデルの定義順で `steps` と同じ数) があれば、
   `update(steps[i])` の代わりに受け手の口 (C ABI の `ugoe_puppet_follow`・wasm の `follow`・Unity の `Puppet.Follow`・Godot の `follow`) で、`follow[i]` と
   `follow_lip_sync[i]` (リップシンクの `[音量, 強さ]`。無いか `null` なら当てない) と `follow_part_opacities[i]`
   (モーションのパーツの不透明度のカーブ `[パーツの番号, 値, 重み]` の並び。無ければ空) と `steps[i]` を渡す (`plan.txt` では
   `follow <dt> <音量 (無ければ -1)> <強さ> <パーツの数 k> (<番号> <値> <重み>) × k <値> ...` の行。T0133・T0134・T0135)。このケースの `effects` は常に `false`。
   計画に `conduct` (T0454) があれば、`follow` の値を読まずに、実行器の中で指揮役を進める: 同じモデル・モジュール・`parameters` で
   もう 1 つ作り、`conduct.effects` (効果の設定の JSON。標準の目パチと呼吸) があれば効果の設定に書き、物理の段を止め、目パチと呼吸は
   `conduct.effects` があるときだけ動かし、`conduct.motion` があれば普通の優先度で再生する。各フレームで `conduct.lip_sync[i]` (あれば)
   を音量にして指揮役を `update(steps[i])` で進め、指揮役の口 (C ABI の `ugoe_puppet_values_before_physics`・`ugoe_puppet_lip_sync_applied`・
   `ugoe_puppet_part_opacities_applied`、wasm の `values_before_physics`・`lip_sync_applied`・`part_opacities_applied_*`、Unity の
   `ValuesBeforePhysics`・`LipSyncApplied`・`PartOpacitiesApplied`、Godot の `get_values_before_physics` など) で読んだ値で受け手の口を呼ぶ
   (`plan.txt` では `conduct [<モーションの名前>]`・`conduct_effects <JSON>`・`lead <dt> <音量 (設定しなければ -1)>` の行)。
   参照は計画の `follow` の値 (参照の側で同じ指揮役を進めて作ったもの) で進めるので、指揮役の口が返す値と受け手に渡す形が合っていることを確かめる。
   受け手の口を持つバインディングは `src/main.rs` の `FOLLOW_BINDINGS` に挙げる (今はすべて: C ABI・wasm・Unity の `Puppet.Follow` (T0455)・Godot)。
   挙げていないバインディングは比べ合わせが `skip` にする。
   計画に `motions` (T0462) があれば、各 `update(steps[i])` の直前に `step == i` のモーションを書かれた順に `priority`
   (`idle`・`normal`・`force`) で始め (始まったかが `accepted` と違えば失敗)、各 `update` の後に発火したモーションのイベントと出どころ
   (C ABI の `ugoe_puppet_motion_event`・`ugoe_puppet_motion_event_source`、wasm の `motion_events_json`、Unity の `MotionEvents`、
   Godot の `get_motion_events`) を結果の `motion_events` に `{"step", "value", "time", "motion", "number"}` で足す
   (`plan.txt` では `update` の行より前の `start <step> <priority> <0|1> <名前>` の行)。比べ合わせは、参照に `motion_events` があれば
   バインディングにも求め、値・名前・始めた番号・`step` の一致と時刻 (1e-4) を比べる。
   `before` があるとき、値を変えてから描画に反映するまでにフレームの遅れが入りうる実行器 (Godot) は、先に `before` の値で
   数フレーム描き、次のフレームで `parameters` にして上の評価をし、その直後のフレームを結果にする。同期して評価・描画する実行器
   (C ABI・wasm・Unity の C# の層) と参照は `before` を使わない (結果は同じでなければならない)。`plan.txt` には書かない。
4. 結果を書く。形は `src/frame.rs` の `Frame` で、公開仕様の評価の例 (`expected.json` のケース) と同じ
   (`{"meshes": {ID: {"positions": [[x, y], ...], "opacity", ...}}, "render_order": [ID, ...]}`)。取り出せない値は省いてよい
   (`positions` と `render_order` は必須)。評価の結果を読み戻せないなら書かず、比べ合わせを `--images-only` にする。
5. 画像を描くなら: キャンバスと同じ大きさ (倍率 1、`round`)、キャンバス全体が収まるビュー、背景 `plan.background` (`#808080`、不透明)、
   `lighting` の光源で描き、ストレートアルファの PNG に書く。
6. 扱えないケースは `skipped.json` に理由と一緒に書く。

### 許容誤差

- 頂点など: 公開仕様の評価の適合の基準 ([spec/evaluation.md#conformance](../../spec/evaluation.md#conformance)、
  `docs/spec/evaluation.md`)。位置は各座標 0.01 px、不透明度・色・マスクの効き具合は 1e-4、描画順の値は 1e-3。
  描画の順番は、参照の描画順の値の差が 1e-3 以下のメッシュの間では問わない。参照の不透明度が 1e-4 以下のメッシュの `visible` は問わない。
- 画像: 描画の互換の基準 ([spec/rendering.md#conformance](../../spec/rendering.md#conformance))。すべてのチャンネルの差が 3/255 以下の画素が 99% 以上。
  ただし透明な黒ではなく**不透明な背景 `#808080`** の上で比べる (ブラウザの canvas の読み戻しはアルファを外して 8 ビットに丸めるので、
  透明な画素の色が比べられない。Godot の `build.mjs --conformance` と同じ背景)。

T0418 の時点の結果 (WSL2、53 ケース): C ABI・wasm (WebGL2 と WebGPU)・Unity の C# の層はすべてのケースで評価の結果が一致。
wasm の画像は WebGL2・WebGPU (どちらも SwiftShader) とも 99.9% 以上の画素が一致し、違いの最大は見本で 5〜7/255 (三角形の縁。
回転の例 2 つは 1〜2 画素だけ 127/255)。Godot (Compatibility、Mesa の llvmpipe) は扱う 20 ケースで 100% の画素が 3/255 以内 (最大 3/255)。

T0420 で Godot の評価の結果の読み戻しとモデルの JSON の読み込みを足した (WSL2、53 ケース): Godot はすべてのケースで評価の結果が一致し、
画像は 100% の画素が 3/255 以内 (見本・マスク・合成モードで最大 2〜3/255、公開仕様の評価の例は 0〜1/255)。モデルの JSON は
テクスチャが無いので、`ugoe render` と同じく不透明な白で描く。

## `<ugoe-avatar>` のシナリオ

`avatar` は、ほかのバインディングと違って参照とは比べない。`bindings/web/src/avatar.test.ts` が happy-dom と `WebPuppet` の偽物で確かめている
作成・切り離し・作り直しの順序が、本物の wasm と GPU (ヘッドレスの Chrome の WebGL2、SwiftShader) でも通ることを確かめる (T0440)。
`run.mjs` のサーバーが [`web/avatar.html`](web/avatar.html) を配り、[`web/avatar.js`](web/avatar.js) がシナリオを順に動かす。

- モデルは見本の `assets/samples/avatar.ugoe` と `assets/samples/dress-up/dress-up.ugoe` (サーバーの `/samples/`。`?delay=ミリ秒` で応答を遅らせ、
  取得中の操作を確かめる)。どちらのモデルかは、描画するメッシュの ID の並びで見分ける (最初に要素を使わずに `WebPuppet` で読んで求める)。
- `WebPuppet.create` と `free` を包んで生きている `WebPuppet` を数え、作成中のものが終わった後に数を確かめる (GPU の資源が残らないこと)。
  各シナリオの後で要素を外し、0 個に戻らなければそのシナリオの失敗にする。
- 描いたことは、要素の shadow root の canvas を 2D の canvas に写し、背景でない画素が 1% を超えるかで確かめる。

| シナリオ | 確かめること |
| --- | --- |
| `load-and-draw` | `src` を読んで `load` (`detail.backend` が `webgl2`) を送り、`ready` が `puppet` で解決し、描く。`src` を変えると読み直す。外すと解放する |
| `context-loss` | `WEBGL_lose_context` の `loseContext()` の間は作り直さず `error` も送らない。`restoreContext()` で新しい canvas に作り直し (古い canvas は外す)、モデルと `setParameter` の値が戻って描く。2 回続けて行う |
| `context-loss-timeout` | `loseContext()` して復元させないと、`gpulost` を 1 回送って `waitingForGpu` が真になり、`ready` が新しい Promise に変わる。時間切れ (`RESTORE_TIMEOUT_MS` = 5 秒) の前は作り直さず、時間切れの後に新しい canvas で作り直して (`load`)、`ready` がその `WebPuppet` で解決し、モデルと値が戻って描く (T0938・T0479) |
| `context-loss-during-load` | `loseContext()` の間に `load()` を呼ぶと、`device_lost` で失敗とせず `error` も送らずに待ち、`restoreContext()` で作り直した後にそのモデルを読んで解決する (前のモデルのパラメータの値は引き継がない)。`loseContext()` の間に `src` の取得が終わっても、作り直した後に前のモデルでなくその `src` を読む (T0486) |
| `gpu-lost-repeatedly` | 作り直しで作れない (GPU が戻らない) とき: 失って戻すと作り直し (1 回目) で作れず、`error` を送らずに待ち、5 秒後の試し直し (2 回目) で作って描く。次の喪失の作り直し (3 回目 = 上限の `MAX_RESTARTS`) でも作れなければ、5 秒後の試し直しで上限を超えて諦め、`error` を 1 回送って待っていた `ready` を reject する (`WebPuppet` も canvas も残らない)。`restart()` で作り直して、モデルと値が戻って描く (T0479) |
| `detach-while-creating` | 作成中に外して戻すのを繰り返しても、`WebPuppet` は 1 つだけ残り、canvas も 1 つで、描く。外したまま作成が終わっても残らない |
| `detach-while-fetching` | 新しい `src` の取得中に外して戻すと、前のモデルではなく新しい `src` を読んで描く |
| `backend-change` | `backend` を変えると作り直してパラメータの値を引き継ぐ (`auto` はこの Chrome では WebGPU のアダプタが無く、同じ canvas の WebGL2 で描く)。取得中・作成中に続けて変えても、`WebPuppet` は 1 つで新しい `src` を描く |
| `gpu-process-crash` | 描いている間に GPU プロセスが落ちると、WebGL2 のコンテキストの喪失として `gpulost` を送って復元を待ち、新しい canvas で作り直して (`load`)、`ready` がその `WebPuppet` で解決し、モデルと値が戻って描く。`error` は送らない。2 回続けて行う (T0484) |
| `gpu-process-crash-during-create` | 要素を付けると同時に時機をずらして (0〜500 ミリ秒後に) GPU プロセスを落とす。wasm は panic せず、要素は作り直して描くか、コードを持つ例外 (`CreateError`・`LoadError`・`RenderError`) の `error` を送って止まる。止まったら `restart()` で作り直して描く。8 回行い、どう終わったかを数えてログに書く (T0484) |

- 作り直しで作れないこと (GPU が戻らない・コンテキストの上限) は、要素が作る新しい canvas のコンテキストを `WebPuppet.create` の前に
  `WEBGL_lose_context` で失わせて起こす (本物の wasm が `CreateError` の `context_unavailable` で失敗する。T0475)。
- GPU プロセスは、ページが `run.mjs` のサーバーに `/crash-gpu` を送り、`run.mjs` がブラウザの CDP の `Browser.crashGpuProcess` で落とす
  (`{"delay": ミリ秒}` を付けると、待たずに応答してからその後で落とすので、wasm の同期的な作成の途中にも落とせる。時機は決まらない)。
  CDP は `--remote-debugging-port=0` で開き、ポートはプロファイルの `DevToolsActivePort` から読んで WebSocket で繋ぐ。Chrome は GPU
  プロセスが続けて落ちると GPU を諦め (SwiftShader の後に代わりが無いとブラウザごと終わる)、落ちたときに WebGL を使っていたページの
  WebGL を止めるので、`avatar` だけ `--disable-gpu-process-crash-limit`・`--disable-domain-blocking-for-3d-apis` で起動する
  (packages/player/browser-test と同じ起動の設定。T0944)。GPU プロセスを落とすシナリオは最後に動かす。
- wasm の panic (`console_error_panic_hook` がコンソールに書く `panicked at`) が 1 つでもあれば、そのシナリオは失敗する (T0484)。
- `gpu-process-crash-during-create` で落ちる時機は決まらないので、通っても全ての段で落としたとは言えない (どの段でも panic しないことの
  決定的な確かめは packages/player/browser-test の `lose-context-during-create`・`invalid-block-index-during-create`)。手元の走り
  (WSL2・SwiftShader) では、作成で失敗して `restart()` したもの・作り直したものが出た (読み込みの途中に失ったときは、T0486 から
  `error` を送らずに作り直して読む。決定的な確かめは `context-loss-during-load`)。
- 復元の時間切れは本物の時間 (5 秒) を待つので、`context-loss-timeout` は約 5 秒、`gpu-lost-repeatedly` は約 10 秒かかる
  (T0479 で測った。`avatar` 全体は 12 秒ほどから 30 秒ほどになった)。
- 作り直しを繰り返すと、捨てた canvas の WebGL のコンテキストも GC されるまで Chrome の上限 (16) に数えられ、今のコンテキストが失われる
  ことがあるので、`run.mjs` は Chrome を `--max-active-webgl-contexts=256` で起動する (packages/player/browser-test と同じ)。

WebGPU のデバイスの喪失 (`is_lost`) はページから起こせないので、ここでは確かめていない (偽物のテストだけ)。

## 問い合わせの JSON のキー

`queries` は、npm パッケージの `WebPuppet` の `*_json()` と `sample_motion_json`・`MotionSampler.sample` の出力のキーを、パッケージの型
(`bindings/web/src/queries.ts`) のキーと比べる (T0463)。型は実行時に残らないので、型ごとのキーの一覧 `QUERY_KEYS`
(`bindings/web/src/query-keys.ts`。`dist/query-keys.js` を `/pkg/` から読む) と比べ、一覧と型が同じことは bindings/web の
`pnpm check` (`src/query-keys.test.ts`) が確かめる。`run.mjs` のサーバーが [`web/queries.html`](web/queries.html) を配り、
[`web/queries.js`](web/queries.js) が項目を順に動かす。

| 項目 | 読むもの |
| --- | --- |
| `no-model` | モデルの無い `WebPuppet` (空の一覧) |
| `sample-motion` | 3 種類の対象 (`parameter`・`part_opacity`・`effect_group`) のカーブを持つモーションを `sampleMotion` (`sample_motion_json`) と `MotionSampler` で評価する |
| `motion-events` | 見本のアバターで、最初の `update` で発火するイベントを持つモーションを `start_motion_json` で再生し、`motion_events_json` を出させる (T0462) |
| `avatar`・`avatar-lit` | 見本のモデル (モーション・表情・アバタープロファイル・パラメータのグループ・ポーズ) |
| `dress-up` | 着せ替えの見本 (パラメータのグループ・ポーズ)。`long-hair`・`cat-ears`・`sailor` を付け、`twin-tails` は読むだけ (`attached: false`) |
| `coverage` | 上のどれかで、一覧のすべての形を 1 度は比べたか |

- 無い必須のキーと、一覧に無いキー (型に無いキー) を食い違いとする。型が `[key: string]: unknown` を持つ形 (パラメータ・スロット) でも、
  このランタイムの出力に型に無いキーがあれば落とす (型に足し忘れたキーを見つけるため)。
- 読み込みの警告 (`load_warnings_json`・`detached_modules_json`・`modules_json` の `warning_details`) の `code` と補足の値は
  `crates/ugoe-wasm/codes.json` で突き合わせているので比べない。物理 (`physics_json`) は型を出していないので比べない。
- 出力の形は描画のバックエンドによらないので、WebGL2 だけで動かす。

## エンジン向けのバインディングを足す

1. 実行器を書く: `plan.json` (か `plan.txt`) を読み、上の「実行器の約束」のとおりに評価して `DIR/<名前>/<ケース>.json` を書く。
   描画できるなら `<ケース>.png` も書く。モデルの読み込み・モジュール・パラメータ・段の有効無効・`update` は、
   そのバインディングの公開 API で行う (内部の `ugoe` を直接呼ばない。配布物として検査するため)。
2. `run.mjs` の `BINDINGS` に名前を足し、道具の検査 (`need`)・段 (ビルド → 実行器 → `compare("<名前>", "frames" | "images" | "images-only")`) を足す。
   画像を比べるなら `IMAGES` に、道具が重く既定では飛ばしてよいなら `OPTIONAL` に入れる。
3. CI: 実行器に要るもの (エンジンの実行ファイルなど) を `conformance` ジョブに入れるか、別のジョブで
   `just conformance --only <名前>` を実行する。
4. この README の表に足す。

### Unreal (`bindings/unreal`、T0407)

まだ無い。できたら Unity と同じく、エンジンに依存しない層があればそれで頂点を、エンジンで描けるなら画像を比べる。

## 見本の MediaPipe の利用状況の送信

`example` は、npm パッケージの見本 ([`bindings/web/examples/basic`](../../bindings/web/examples/basic/index.html)) の「camera (MediaPipe)」が
Face Landmarker を作った (`FaceLandmarker.createFromOptions` の) 後に、MediaPipe の利用状況の送信 (`https://odml.pa.googleapis.com/v1/log`。
[bindings/web/README.md](../../bindings/web/README.md#mediapipe-の利用状況の送信)) がネットワークに出ないことを確かめる (T0487)。
見本は `refuseMediaPipeUsageLog()` で `fetch` を包み、送り先への要求を断る (T0947)。[`example.mjs`](example.mjs) が動かす。

- **配り方:** 見本は書き換えずに `run.mjs` (example.mjs) のサーバーが配る。import map の `./node_modules/@ugoe/runtime/` は bindings/web
  (`dist/`)、`/samples/` は `assets/samples` を指す (tgz を作らない)。`?backend=webgl2` で開く。
- **ネットワークに頼らない:** 見本が CDN から読む MediaPipe (jsdelivr の `@mediapipe/tasks-vision`) とモデル (storage.googleapis.com) は、
  CDP の `Fetch.enable` (`https://*`) で止めて手元のファイルで応える。MediaPipe は packages/face-tracking が入れている
  `@mediapipe/tasks-vision` (見本の `MEDIAPIPE` の版と違えば止まる)、モデルは packages/face-tracking の `mediapipe-assets.mjs` が取得して
  SHA-256 を確かめたもの (`target/conformance-mediapipe/`。`target/conformance/` は毎回消すのでその外に置き、取得済みで SHA-256 が合えば取り直さない)。ほかの外への要求は断り、警告に出す。
- **カメラ:** Chrome の偽の映像 (`--use-fake-device-for-media-stream`・`--use-fake-ui-for-media-stream`)。顔は映らないが、
  `detectForVideo` は動く (見本の `cameraFrames` が 1 以上になるまで待つ)。
- **ネットワークの記録:** ページに `Target.attachToTarget` (`flatten`) で繋ぎ、`Network.enable` の `Network.requestWillBeSent` で
  ページが出した要求をすべて記録する (Fetch で止めた要求も記録される)。`fetch` の包みが断った要求はネットワークの層に届かないので
  記録されない。送り先への要求が 1 件でもあれば失敗。
- **時間を縮める:** MediaPipe は Landmarker を作ると、60 秒ごとの `setInterval` で送る (作ったときの記録が溜まっているので、最初の間隔で送る)。
  ページを開く前に (`Page.addScriptToEvaluateOnNewDocument`) 60000 ミリ秒の `setInterval` を 1 秒に縮める。
- **2 回開く:**
  - `guarded` (見本のまま): 見本が送信を断った回数 (`window.ugoeExample.usageLogsRefused`) が 1 以上になるのを待ち (送ろうとしたことの確かめ)、
    さらに 3 秒待って、送り先への要求が記録されていないことを確かめる。
  - `unguarded` (対照): `refuseMediaPipeUsageLog()` は `fetch` がすでに包んだものなら何もしないので、ページを開く前に `fetch` に包んだ印
    (`refusesMediaPipeUsageLog`) を付けて包みを外し、送り先への要求が記録されることを確かめる (このテストが送信を見つけられることの確かめ)。
    要求は Fetch で止める (`BlockedByClient`) ので、実際には Google に送らない (手元では 1 回の送信で 2 件記録された)。
- 見本の `refuseMediaPipeUsageLog()` の呼び出しを外すと `guarded` が落ちる (T0487 で確かめた)。
- `@mediapipe/tasks-vision` の版を上げて送り方が変わったこと (送信の経路・間隔) は、packages/face-tracking の
  `mediapipe-bundle.test.ts` (T0947) が見る。ここでは `guarded` で断った回数が 0 のまま 20 秒たつと落ちる。

## 確かめていないこと・制限

- **CI の上では、T0418 で足したもの (wasm の WebGPU・Unity・Godot の計画への移行) を動かしていない** (push しないため)。
  wasm の WebGPU はヘッドレスの Chrome の SwiftShader の Vulkan (`--use-webgpu-adapter=swiftshader`) で、WSL2 の Chrome 154 では通った。
  T0408 の時点では canvas の読み戻しが空になる・デバイスを失う・アダプタが取れなくなることがあったが、`read_pixels`
  (別のテクスチャに描いて読み戻す) に変えてからは起きていない。ランナーの Chrome で起きたら `--only c,wasm,unity` で外せる。
- **Unity:** Unity に依存しない C# の層 (P/Invoke と C ABI) だけを比べる。Unity のメッシュ・シェーダ・マスクの描画は比べていない
  (Unity のライセンスとエディタが要る。`bindings/unity/README.md#制限と確かめていないこと`)。
- **Godot:** 比べ合わせは Compatibility (OpenGL) だけ。評価の結果は画像を読み戻すのと同じフレームでノードから読み戻す。
- Windows と macOS では実行していない (`run.mjs` はリンクの引数だけ用意してある)。

## 関連タスク

- T0408 (`docs/tasks/T0408-conformance-tests.md`)
- T0418 (`docs/tasks/T0418-conformance-more-backends.md`: Godot と Unity・wasm の WebGPU)
- T0420 (`docs/tasks/T0420-unity-and-conformance-extras.md`: Godot の評価の結果の読み戻しとモデルの JSON)
- T0133 (`docs/tasks/T0133-runtime-explicit-base-values.md`: 受け手として進めるケース (`follow`))
- T0455 (`docs/tasks/T0455-unity-follow.md`: Unity の実行器で `follow` のケースを比べる)
- T0222 (`docs/tasks/T0222-ktx2-decode-headless-and-wasm.md`: KTX2 のテクスチャのケース)
