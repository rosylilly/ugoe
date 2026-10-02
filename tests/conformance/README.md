# 配布物の適合テスト (`ugoe-conformance`)

**層:** ツール層 (テスト用。依存してよいクレートは `scripts/check-layers.mjs`)

すべての配布物で、同じモデル・同じパラメータから同じ頂点と画像が得られることを検査する
([ADR 0017](../../docs/adr/0017-single-runtime-distribution.md) の決定 3、[docs/portability.md](../../docs/portability.md#配布))。
参照は `ugoe::Puppet` (の `ModelInstance`) と `ugoe render` で作り、各バインディングの出力を公開仕様の許容誤差で比べる。

| 配布物 (`--only` の名前) | 頂点 | 画像 | 実行器 |
| --- | --- | --- | --- |
| C ABI (`ugoe-ffi`、`c`) | 位置・不透明度・描画順の値・乗算色・スクリーン色・マスクの効き具合・`visible`・描画の順番 | なし (C ABI は描画しない) | [`c/runner.c`](c/runner.c) |
| wasm (npm パッケージ `@ugoe/runtime`、`wasm` と `wasm-webgpu`) | C ABI と同じすべての値 (`drawable_positions`・`drawable_states_json`・`render_order`) | あり (ヘッドレスの Chrome の WebGL2 と WebGPU。`read_pixels` で読み戻す) | [`web/runner.js`](web/runner.js) |
| Unity (`bindings/unity` の Unity に依存しない C# の層 `Runtime/Core`、`unity`) | C ABI と同じすべての値 | なし (描画は Unity が要る) | [`unity/Program.cs`](unity/Program.cs) (dotnet) |
| Godot (`bindings/godot`、`godot`) | C ABI と同じすべての値 (ノードの `get_drawable_positions`・`get_drawable_state`・`get_render_order` で読み戻す) | あり (Compatibility (OpenGL)。画面が無ければ Xvfb) | [`bindings/godot/example/tests/conformance.gd`](../../bindings/godot/example/tests/conformance.gd) |

## 実行

```sh
just conformance                          # = node tests/conformance/run.mjs (すべてのバインディング)
just conformance --only c                 # C ABI だけ (Chrome も GPU も要らない)
just conformance --only wasm,wasm-webgpu  # wasm だけ (WebGL2 と WebGPU)
just conformance --only unity             # Unity の C# の層だけ (dotnet と C ABI のライブラリ)
just conformance --only godot             # Godot だけ (= just godot-check の比べ合わせ)
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

CI では `.github/workflows/ci.yml` の `conformance` ジョブが `just conformance` を実行し (ランナーに dotnet があれば Unity も、Godot は無いので飛ぶ)、
`godot` ジョブの `just godot-check` (`build.mjs --conformance`) が `--only godot` を実行する。落ちたら `target/conformance` を成果物として残す。
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
| `generated` | `model` の代わりに、[`src/bundles.rs`](src/bundles.rs) が作るモデルの名前 (`nested-masks`・`mixed-masks`・`intersection-masks`・`blend-modes`・`many-masks`・`moving-masks`) |
| `spec` | `spec/examples/<例>` を書くと、その `expected.json` のケースごとに展開する (`spec-<例>-<番号>`。モデルとパラメータはそこから取る) |
| `modules` | 付ける `.ugoemod` (付ける順) |
| `parameters` | パラメータの値 (値域の外はクランプされる) |
| `before` | 先に描く値 (ID は `parameters` にもあること)。下の「実行器の約束」の 3 |
| `time` / `fps` | 時間を進める秒数と 1 秒あたりの `update` の回数 (既定 60)。`ugoe render --time --fps` と同じ |
| `effects` | 時間を進めるときに目パチと呼吸を使うか (既定 `false`) |
| `follow` | 受け手として進めるか (既定 `false`。T0133)。`time` と `"image": false` が要る。計画は、物理を切った指揮役の `Puppet` (`effects` なら標準の目パチと呼吸を使う) を `time` の列で進めた各フレームの値を `follow` に持ち、実行器は `update` の代わりに `follow(値, dt)` を呼ぶ (下の「実行器の約束」の 3) |
| `lighting` | 光源 (画像だけに効く)。形は `WebPuppet::set_lighting_json` と同じ |
| `image` | 画像も比べるか (既定 `true`) |

入力は見本 (`assets/samples/avatar.ugoe`・`avatar-lit.ugoe`・`dress-up/`)、生成するマスクと合成モードのモデル (もとは Godot の
`build.mjs --conformance` だけにあったもの。T0418 で移した)、公開仕様の評価の例 (`spec/examples/*/expected.json`) と
バンドルの例 (`spec/examples/bundles/`: zstd の圧縮・任意の機能・法線マップ・物理)。描画の例 (`render.json`) は、テクスチャを
その場で作る必要があるので使っていない (参照レンダラは `crates/ugoe-render/tests/spec_render.rs` で確かめている)。

### 実行器の約束 (計画)

`plan.json` の各ケース (`src/plan.rs` の `Case`) と `plan.txt` (C 向けの行の形式。`Plan::to_text`) は同じ内容を持つ。実行器は次のとおりにする。

1. モデルを読む (`bundle` なら `.ugoe`、違えば JSON)。`modules` を順に付ける。
2. `parameters` を設定する。
3. `steps` が空なら、すべての段 (物理・目パチ・呼吸・リップシンク・ポーズ) を止めて 1 回評価する (時間を進めない評価。
   参照は `ModelInstance::update` 1 回で、`ugoe render` の `--time` なしと同じ)。
   `steps` があれば、目パチと呼吸を `effects` のとおりにし、ほかの段は動かして、`steps` の `dt` で順に `update` する
   (`ugoe render --time` と同じ列。実行器では `dt` を計算しない)。
   計画に `follow` (各フレームのパラメータの値。モデルの定義順で `steps` と同じ数) があれば、`update(steps[i])` の代わりに
   受け手の口 (C ABI の `ugoe_puppet_follow`・wasm の `follow`) で `follow(follow[i], steps[i])` を呼ぶ (`plan.txt` では
   `follow <dt> <値> ...` の行。T0133)。このケースの `effects` は常に `false`。受け手の口を持たないバインディング (今は Unity と Godot) は、
   比べ合わせ (`src/main.rs` の `FOLLOW_BINDINGS`) が `skip` にする。
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
