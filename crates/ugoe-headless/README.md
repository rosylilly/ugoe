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
| `args` | 文字列の引数を読む (`ID=VALUE`・`WxH`・色・`INDEX=PATH`・秒数・fps・光の色・光源)。clap の `value_parser` にそのまま渡せる |
| `view` | 出力の大きさ (`output_size`) とキャンバスを収めるビュー (`fit_view`・`Fit`) |
| `textures` | テクスチャを揃える (`--texture` の差し替え → バンドルの PNG → 不透明な白 1x1)・バンドルの法線マップをデコードする |
| `animate` | パラメータの書き込みと、`ugoe::Puppet` で時間を進める処理 |
| `gpu` | ヘッドレスの `Gpu` (アダプタとデバイス)・描いて読み戻す・`stats_json` |
| `image` | PNG の読み書きと乗算済みアルファの変換 |

- **ライティング:** `Scene::from_bundle` がバンドルの法線マップを持ち、`prepare` がデコードして `Prepared::shading` に入れる。
  光源は `RenderOptions::lighting` (`None` ならレンダラに渡さないので、出力は法線マップが無いのと同じ)。
  `Gpu::render` が `Renderer::set_normal_map` と `set_lighting` を呼ぶ。規則は `ugoe render` の節の「ライティング」。
- feature `clap`: `Fit` に `clap::ValueEnum` を付ける (`ugoe-cli` が使う)。
- エラーは `ugoe_headless::Error` (thiserror)。原因 (`source`) をたどると `anyhow` の `Caused by` と同じ情報になる。

## GPU

- サーフェス無しでアダプタを要求する。`Gpu::new(Some("gl"))` のようにバックエンドを選べる (省略時は `WGPU_BACKEND`)。
  GPU の無い環境では Mesa の lavapipe (Vulkan) か llvmpipe (GL) を使う。
- **破棄しない:** WSL2 の dzn (Vulkan on D3D12) はインスタンスやデバイスの破棄時に SIGSEGV を起こすことがあるので、
  CLI は描いたあと `Gpu::leak` で手放してすぐ終了し、MCP サーバは 1 つの `Gpu` を使い回して破棄しない。
- **エラースコープ:** `Gpu::render` は描画中の wgpu のエラー (検証・メモリ不足・内部) をエラースコープで受け取り、
  `Error::Gpu` にして返す (wgpu の既定では panic になる)。

## テスト

```sh
cargo test -p ugoe-headless   # GPU を使わないテスト (引数・大きさ・fit・テクスチャ・時間の進め方・PNG)
```

実際の描画は `ugoe render` のテスト (`cargo test -p ugoe-cli --features gpu-tests`) と、MCP サーバのテスト
(`cargo test -p ugoe-mcp --features gpu-tests`) が確かめる。どちらも `just test-gpu` に含まれる。

## 関連タスク

- T0511 (`docs/tasks/T0511-shared-headless-render.md`)
- T0210 (法線マップと光源を渡す。`docs/tasks/T0210-lighting-hosts.md`)
