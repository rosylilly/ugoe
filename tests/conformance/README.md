# 配布物の適合テスト (`ugoe-conformance`)

**層:** ツール層 (テスト用。依存してよいクレートは `scripts/check-layers.mjs`)

すべての配布物で、同じモデル・同じパラメータから同じ頂点と画像が得られることを検査する
([ADR 0017](../../docs/adr/0017-single-runtime-distribution.md) の決定 3、[docs/portability.md](../../docs/portability.md#配布))。
参照は `ugoe::Puppet` (の `ModelInstance`) と `ugoe render` で作り、各バインディングの出力を公開仕様の許容誤差で比べる。

| 配布物 | 頂点 | 画像 | 実行器 |
| --- | --- | --- | --- |
| C ABI (`ugoe-ffi`) | 位置・不透明度・描画順の値・乗算色・スクリーン色・マスクの効き具合・`visible`・描画の順番 | なし (C ABI は描画しない) | [`c/runner.c`](c/runner.c) |
| wasm (npm パッケージ `@ugoe/runtime`) | 位置・描画の順番 (`WebPuppet` が出すもの) | あり (ヘッドレスの Chrome の WebGL2) | [`web/runner.js`](web/runner.js) |
| Godot (`bindings/godot`) | — | いまは別に `just godot-check` で比べている | [下](#godot-bindingsgodot) |

## 実行

```sh
just conformance                    # = node tests/conformance/run.mjs (C ABI と wasm)
just conformance --only c           # C ABI だけ (Chrome も GPU も要らない)
just conformance --backend webgpu   # wasm を WebGPU で描く (下の「確かめていないこと」)
just conformance --no-web-build     # bindings/web/dist を作り直さない
```

結果は `target/conformance/` (`--out` で変えられる) に残る。

```
target/conformance/
  plan.json  plan.txt          計画 (展開したケース)
  reference/<ケース>.json .png  参照
  c/<ケース>.json               C ABI の結果
  wasm/<ケース>.json .png       wasm の結果。画像が合わなければ <ケース>.diff.png (違う画素が赤)
  chrome.log                   Chrome のログ (ページのコンソールを含む)
```

要るもの (`just ci` に含めない理由): C コンパイラ (`cc`。`CC` で変えられる)、Chrome (`CHROME` で指定。無ければ
`google-chrome`・`chromium` などを探す)、wasm-bindgen-cli (`just wasm-bindgen-version` の版。パッケージのビルド)、
参照の画像を描く GPU (無ければ Mesa の lavapipe / llvmpipe)。`run.mjs` は最初にこれらを cc・Chrome・wasm-bindgen-cli・GPU の順に調べ
(`--only c` では C コンパイラだけ)、足りないものと入れ方を出して止まる。CI では `.github/workflows/ci.yml` の `conformance` ジョブで実行し、
落ちたら `target/conformance` を成果物として残す。
このクレートの単体テスト (ケースの展開・比べ合わせ・参照の評価) は `cargo test` に入るので `just ci` で動く。

## 流れ

1. `ugoe-conformance reference --out DIR`: [`cases.json`](cases.json) を計画に展開し (`DIR/plan.json`・`DIR/plan.txt`)、
   ケースごとに参照の評価の結果を `DIR/reference/<ケース>.json` に書く ([`src/reference.rs`](src/reference.rs))。
2. 画像を比べるバインディングがあれば、`run.mjs` がケースごとに `ugoe render <plan.json の render_args> -o DIR/reference/<ケース>.png` を実行する。
3. 各バインディングの実行器が、計画を読んで `DIR/<バインディング>/<ケース>.json` (と `.png`) を書く。
4. `ugoe-conformance compare --out DIR --binding <名前> [--images]` が比べる ([`src/compare.rs`](src/compare.rs))。

### ケース (`cases.json`)

| フィールド | 内容 |
| --- | --- |
| `name` / `summary` | 名前 (出力のファイル名。英数字・`-`・`_`) と説明 |
| `model` | `.ugoe` かモデルの JSON (リポジトリのルートからの相対パス) |
| `spec` | `spec/examples/<例>` を書くと、その `expected.json` のケースごとに展開する (`spec-<例>-<番号>`。モデルとパラメータはそこから取る) |
| `modules` | 付ける `.ugoemod` (付ける順) |
| `parameters` | パラメータの値 (値域の外はクランプされる) |
| `time` / `fps` | 時間を進める秒数と 1 秒あたりの `update` の回数 (既定 60)。`ugoe render --time --fps` と同じ |
| `effects` | 時間を進めるときに目パチと呼吸を使うか (既定 `false`) |
| `lighting` | 光源 (画像だけに効く)。形は `WebPuppet::set_lighting_json` と同じ |
| `image` | 画像も比べるか (既定 `true`) |

入力は見本 (`assets/samples/avatar.ugoe`・`avatar-lit.ugoe`・`dress-up/`)、公開仕様の評価の例 (`spec/examples/*/expected.json`) と
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
4. 結果を書く。形は `src/frame.rs` の `Frame` で、公開仕様の評価の例 (`expected.json` のケース) と同じ
   (`{"meshes": {ID: {"positions": [[x, y], ...], "opacity", ...}}, "render_order": [ID, ...]}`)。取り出せない値は省いてよい
   (`positions` と `render_order` は必須)。
5. 画像を描くなら: キャンバスと同じ大きさ (倍率 1、`round`)、キャンバス全体が収まるビュー、背景 `plan.background` (`#808080`、不透明)、
   `lighting` の光源で描き、ストレートアルファの PNG に書く。

### 許容誤差

- 頂点など: 公開仕様の評価の適合の基準 ([spec/evaluation.md#conformance](../../spec/evaluation.md#conformance)、
  `docs/spec/evaluation.md`)。位置は各座標 0.01 px、不透明度・色・マスクの効き具合は 1e-4、描画順の値は 1e-3。
  描画の順番は、参照の描画順の値の差が 1e-3 以下のメッシュの間では問わない。参照の不透明度が 1e-4 以下のメッシュの `visible` は問わない。
- 画像: 描画の互換の基準 ([spec/rendering.md#conformance](../../spec/rendering.md#conformance))。すべてのチャンネルの差が 3/255 以下の画素が 99% 以上。
  ただし透明な黒ではなく**不透明な背景 `#808080`** の上で比べる (ブラウザの canvas の読み戻しはアルファを外して 8 ビットに丸めるので、
  透明な画素の色が比べられない。Godot の `build.mjs --conformance` と同じ背景)。

T0408 の時点の結果 (WSL2): C ABI はすべてのケースで一致。wasm (WebGL2、SwiftShader) の画像は 99.9% 以上の画素が一致し、
違いの最大は見本で 4〜7/255 (三角形の縁)。

## エンジン向けのバインディングを足す

1. 実行器を書く: `plan.json` (か `plan.txt`) を読み、上の「実行器の約束」のとおりに評価して `DIR/<名前>/<ケース>.json` を書く。
   描画できるなら `<ケース>.png` も書く。モデルの読み込み・モジュール・パラメータ・段の有効無効・`update` は、
   そのバインディングの公開 API で行う (内部の `ugoe` を直接呼ばない。配布物として検査するため)。
2. `run.mjs` に `--only <名前>` の段を足す (ビルド → 実行器 → `compare("<名前>", 画像を比べるか)`)。
   画像を比べるバインディングがあるときは、参照の画像も作られる (`only.has(...)` の条件に足す)。
3. CI: 実行器に要るもの (エンジンの実行ファイルなど) を `conformance` ジョブに入れるか、別のジョブで
   `just conformance --only <名前>` を実行する。
4. この README の表に足す。

### Godot (`bindings/godot`)

いまは `just godot-check` (`node bindings/godot/build.mjs --conformance`) が、`ugoe render --time 0` の画像と Godot で描いた画像を
別に比べている (`example/tests/conformance.gd`。同じ基準・同じ背景 `#808080`)。ケースは `build.mjs` の中にあり、形
(`model`・`modules`・`params`・光源) はこの計画に近い。この適合テストに乗せるには:

1. `conformance.gd` を実行器にする: 計画 (`plan.json`) を読み、ケースごとに `UgoePuppet2D` を作って `<ケース>.png` (と、取り出せれば
   `<ケース>.json`) を書く。比べ合わせは `ugoe-conformance compare --binding godot --images` に任せ、GDScript での比べ合わせは外す。
   時間は `playing = false` のまま `advance(dt)` を `steps` の順に呼ぶ。
2. 時間を進めないケース (`steps` が空) は、すべての段を止めた評価になっている。Godot のノードは読み込みの後に必ず `update(0)` を通す
   (物理も初期化される) ので、段の有効無効をノードに出すか、Godot では `steps` のあるケースだけを使う
   (`build.mjs` が `--time 0` で揃えているのと同じ)。
3. `build.mjs` にだけある条件を `cases.json` に移す: `conformance_bundles` のモデル (入れ子・反転・共通部分のマスク、合成モード、
   マスク 12 個) はバンドルとして置き、値を変えた直後のフレームを比べる `before` は、計画に「先に描く値」のフィールドを足して表す。
4. `run.mjs` に `--only godot` を足し (Godot と、画面が無ければ xvfb-run が要る)、CI の `godot` ジョブで `just conformance --only godot` を実行する。

## 確かめていないこと・制限

- **wasm の WebGPU:** `--backend webgpu` はヘッドレスの Chrome (SwiftShader の Vulkan) で、canvas の読み戻しが空になる・
  途中で GPU のデバイスを失う・アダプタが取れなくなる、のどれかになった (Chrome 154、WSL2。`chrome.log` に
  `SharedImageManager::ProduceMemory` の誤り)。画像の比べ合わせは WebGL2 だけで行っている。WebGPU を確かめるには、
  wasm に描画先のテクスチャを読み戻す API を足すのがよい (後続の候補)。
- wasm の頂点は位置と描画の順番だけ (`WebPuppet` は不透明度などを出さない)。
- Windows と macOS では実行していない (`run.mjs` はリンクの引数だけ用意してある)。

## 関連タスク

- T0408 (`docs/tasks/T0408-conformance-tests.md`)
