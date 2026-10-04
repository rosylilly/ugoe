# ugoe-headless

**層:** ツール層 (ランタイム層だけに依存する。依存してよいクレートは `scripts/check-layers.mjs` を参照)

ヘッドレス描画。`ugoe render` (`ugoe-cli`) と MCP サーバの `render_png` (`ugoe-mcp`) が同じ関数を呼ぶので、
描画の規則 (出力の大きさと `fit`・背景・パラメータのクランプ・テクスチャ・時間の進め方) が 2 つでずれない。
規則そのものは [crates/ugoe-cli/README.md の `ugoe render`](../ugoe-cli/README.md#ugoe-render)。

## 使い方

```rust
use ugoe_headless::{Gpu, RenderOptions, Scene, image, prepare};

let scene = Scene::from_bundle(ugoe_format::read_bundle(&bytes)?);
let options = RenderOptions { params: vec![("AngleX".into(), 10.0)], width: Some(512), ..Default::default() };
// GPU を使う前に引数を検査し、パラメータを書き込み、必要なら時間を進める。
let prepared = prepare(scene, &options)?;
let gpu = Gpu::new(None)?;                 // バックエンドは WGPU_BACKEND か wgpu の既定
let (rgba, stats) = prepared.render(&gpu)?; // 乗算済みアルファの RGBA8 と FrameStats
let png = image::encode_png_unpremultiplied(&rgba)?;
gpu.leak();                                // 破棄しない (下の「GPU」)
```

| モジュール | 内容 |
| --- | --- |
| `lib.rs` | `Scene` (モデルとバンドルの中身)・`RenderOptions` (`ugoe render` のオプションと 1 対 1)・`prepare`・`Prepared::render` |
| `args` | 文字列の引数を読む (`ID=VALUE`・`WxH`・色・`INDEX=PATH`・秒数・fps・GPU を待つ秒数・光の色・光源)。clap の `value_parser` にそのまま渡せる |
| `view` | 出力の大きさ (`output_size`。大きさを指定しないときに上限へ縮める倍率は `shrink_scale`。`RenderOptions::shrink_to` で使う) とキャンバスを収めるビュー (`fit_view`・`Fit`) |
| `textures` | テクスチャを揃える (`--texture` の差し替え → バンドルの PNG / KTX2 → 不透明な白 1x1)・バンドルの法線マップをデコードする |
| `animate` | パラメータの書き込みと、`ugoe::Puppet` で時間を進める処理 (`update` は `MAX_UPDATES` 回 = 60 fps で 1 時間まで) |
| `gpu` | ヘッドレスの `Gpu` (アダプタとデバイス)・描いて読み戻す・`stats_json` |
| `resident` | 常駐の描画の口 `Resident` (毎フレーム描き続ける。下の「常駐の描画」) |
| `yuv` | 画素の形 `PixelFormat` (RGBA8・NV12・YUY2) と、RGBA からの変換の CPU の参照実装 |
| `image` | PNG の読み書き・KTX2 の読み込み (圧縮していない 8bit の RGB / RGBA) と乗算済みアルファの変換 (デコードと一辺の上限は `ugoe_format::image` を ugoe-wasm と共有)。適合テスト用の KTX2 の書き出し `encode_ktx2` (乗算済みの印を立てない) |

- **ライティング:** `Scene::from_bundle` がバンドルの法線マップを持ち、`prepare` がデコードして `Prepared::shading` に入れる。
  光源は `RenderOptions::lighting` (`None` ならレンダラに渡さないので、出力は法線マップが無いのと同じ)。
  `Gpu::render` が `Renderer::set_normal_map` と `set_lighting` を呼ぶ。規則は `ugoe render` の節の「ライティング」。
- feature `clap`: `Fit` に `clap::ValueEnum` を付ける (`ugoe-cli` が使う)。
- エラーは `ugoe_headless::Error` (thiserror)。原因 (`source`) をたどると `anyhow` の `Caused by` と同じ情報になる。

## 常駐の描画

`Gpu::render` (`Prepared::render`) は 1 枚を描く口で、呼ぶたびにレンダラ・テクスチャ・描画先を作り直して読み戻しを待つ。
スタジオの仮想カメラのように毎秒 30〜60 回描くときは `Resident` を使う (ADR 0028 の 3.)。

```rust
let mut resident = Resident::new(&gpu, &textures, &shading, ResidentOptions {
    size: (1920, 1080), format: PixelFormat::Nv12, background: [255, 255, 255, 255],
    view: fit_view(canvas, (1920, 1080), Fit::Contain), slots: DEFAULT_SLOTS, cpu_yuv: false,
})?;
// 毎フレーム: 描いて読み戻しを頼む (待たない)。空いたバッファが無ければ false (GPU が追いついていない)。
resident.submit(&gpu, puppet.instance(), seq)?;
// 終わったものから最も新しい 1 枚 (古いものは捨てて superseded() に数える)。
if let Some(frame) = resident.poll(&gpu, Wait::No)? { /* frame.bytes は NV12 */ }
```

- レンダラ・テクスチャ・法線マップ・描画先・読み戻しのバッファ (既定 3 枚) を持ち続ける。モデルが変わったら作り直す。
- NV12・YUY2 への変換は GPU の計算シェーダ (`src/yuv.wgsl`) で行ってから読み戻す (読み戻す量は RGBA の 3/8〜1/2)。
  計算シェーダの無いアダプタでは RGBA を読み戻して CPU で変換する (`yuv::convert`。式は同じ)。
  `ResidentOptions::cpu_yuv` を `true` にすると、計算シェーダがあっても CPU で変換する (この道をテストで通すため。`Resident::converts_on_gpu` で確かめられる)。
- 色の変換は BT.709 の限定範囲。YUV の形は幅を 4 の倍数、高さを 2 の倍数にする。
- アルファは見ないので、クリアの色 (`background`) は不透明にする (仮想カメラは透過を出せないので単色で埋める)。
- `set_lighting` で光源を変え、`set_backdrop` で背景の画像 (出力と同じ大きさの乗算済み RGBA8) を置く。背景の画像があれば、クリアの代わりに
  毎フレームその画を描画先に写してから描く (GPU の中の写し。スタジオの仮想カメラがシーンの背景の画像に使う。T0808)。
  `image::cover` は CSS の `background-size: cover` と同じ置き方で画像を出力に合わせ (バイリニア)、`image::over_color` は透過のところを色で埋める。

## GPU

- サーフェス無しでアダプタを要求する。`Gpu::new(Some("gl"))` のようにバックエンドを選べる (省略時は `WGPU_BACKEND`)。
  GPU の無い環境では Mesa の lavapipe (Vulkan) か llvmpipe (GL) を使う。
- **破棄しない:** WSL2 の dzn (Vulkan on D3D12) はインスタンスやデバイスの破棄時に SIGSEGV を起こすことがあるので、
  CLI は描いたあと `Gpu::leak` で手放してすぐ終了し、MCP サーバは 1 つの `Gpu` を使い回して破棄しない。
- **エラースコープ:** `Gpu::render` は描画中の wgpu のエラー (検証・メモリ不足・内部) をエラースコープで受け取り、
  `Error::Gpu` にして返す (wgpu の既定では panic になる)。
- **待つ時間の上限:** `Gpu::render` と `Resident::poll(Wait::All)` は、読み戻しを `Gpu::wait_timeout` (既定は
  `DEFAULT_WAIT_TIMEOUT` = 30 秒。ソフトウェアのラスタライザでも 1 枚は数秒かからないので、十分に余裕を持たせた長さ) までしか待たず、超えたら
  `Error::Gpu` を返す (`Error::is_gpu_wait_timeout` が `true`)。ドライバが止まっても呼び出したスレッドが戻らなくならないように。
  上限は `Gpu::set_wait_timeout` で変えられる (`Duration::MAX` なら上限なし。`ugoe render` と `ugoe-mcp` の `--gpu-timeout` は `args::parse_gpu_timeout` で読む)。上限を超えた `Gpu` (とそれで作った `Resident`) は
  止まっているかもしれないので使い回さず、破棄もせずに手放して作り直す (MCP サーバがそうする)。
  wgpu の GL のバックエンドは 1 回の待ちを約 2.1 秒で打ち切るので、1 秒ずつ待ち直して上限まで待つ。

## テスト

```sh
cargo test -p ugoe-headless   # GPU を使わないテスト (引数・大きさ・fit・テクスチャ・時間の進め方・PNG)
```

常駐の描画は `cargo test -p ugoe-headless --features gpu-tests` (`tests/resident_gpu.rs`。RGBA が 1 枚の口と同じ・
GPU の YUV が CPU の参照と ±1 で一致・`cpu_yuv` の CPU の道が GPU の道と ±1 で一致・読み戻しを待たない) が確かめ、`just test-gpu` に含まれる。

実際の描画は `ugoe render` のテスト (`cargo test -p ugoe-cli --features gpu-tests`) と、MCP サーバのテスト
(`cargo test -p ugoe-mcp --features gpu-tests`) が確かめる。どちらも `just test-gpu` に含まれる。

## 関連タスク

- T0511 (`docs/tasks/T0511-shared-headless-render.md`)
- T0210 (法線マップと光源を渡す。`docs/tasks/T0210-lighting-hosts.md`)
- T0807 (常駐の描画の口と YUV への変換。スタジオの仮想カメラ)
