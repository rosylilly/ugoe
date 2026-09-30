---
id: T0408
title: 配布物の適合テスト
status: done
milestone: M7
crates: [tests/conformance, CI, Cargo.toml, Cargo.lock, scripts, commitlint.config.mjs, biome.json, justfile, .github/workflows, docs, AGENTS.md]
depends_on: [T0401, T0404]
assignee: claude-worker-T0408
---

# T0408: 配布物の適合テスト

## 目的

すべての配布物で、同じモデル・同じパラメータから同じ頂点と画像が得られることを検査する。

## 受け入れ条件

- [x] 参照となる出力 (頂点と画像) を `ModelInstance` と `ugoe render` から生成する
- [x] C ABI と wasm で比較するテストが CI で動く
- [x] エンジン向けのバインディングが追加されたら同じテストに乗せる手順を書く
- [x] `just ci` が通る

## 参考

- `docs/adr/0017-single-runtime-distribution.md`

## メモ

- **やったこと:** `tests/conformance` にクレート `ugoe-conformance` (ツール層、publish しない) を置いた。
  `cases.json` (見本の avatar・avatar-lit・dress-up と、公開仕様の評価の例 8 つを `expected.json` のケースごとに展開したもの、
  バンドルの例 4 つ。計 46 ケース) を計画 (`plan.json`・C 向けの `plan.txt`) に展開し、参照の評価の結果を `ugoe::Puppet`
  (`ModelInstance`。時間を進めるケースは `ugoe render --time` と同じ `update` の列) から、参照の画像を `ugoe render` から作る。
  C ABI の実行器 (`c/runner.c`) と wasm の実行器 (npm パッケージの dist をヘッドレスの Chrome で動かす `web/runner.js`) の結果を、
  公開仕様の評価の許容誤差 (位置 0.01 px など) と描画の互換の基準 (3/255 以内が 99%。背景は不透明な `#808080`) で比べる。
  `just conformance` (= `node tests/conformance/run.mjs`) で実行し、CI は新しい `conformance` ジョブ (Mesa の lavapipe・
  ランナーの Chrome・wasm-bindgen) で実行する。C コンパイラ・Chrome・GPU が要るので `just ci` には入れていない
  (ケースの展開・比べ合わせ・参照の評価の単体テストは `cargo test` で `just ci` に入る)。
- **結果 (WSL2):** C ABI は 46 ケースすべて一致。wasm (WebGL2、SwiftShader) も 46 ケースすべて一致し、画像は 99.9% 以上の画素が一致
  (違いの最大は見本で 4〜7/255、三角形の縁)。比べ合わせが違いを検出することは、物理のケースに別のケースの結果を置いて確かめた。
- **範囲の追加 (team-lead 承認済み):** 次を crates: に足した。
  - ルートの `Cargo.toml` (members に `tests/conformance`) と `Cargo.lock`: 参照を `ugoe::Puppet` と `ugoe-headless` で作るクレートを
    ワークスペースに入れ、`cargo test` (`just ci`) で単体テストを動かすため。
  - `scripts/check-layers.mjs`: 新しいクレートをツール層 (ランタイム層 + ugoe-headless に依存してよい) として登録するため。
  - `commitlint.config.mjs`: スコープ `conformance` を足すため。
  - `biome.json`: `tests/**` の JS と JSON を Biome の検査の対象にするため。
  - `justfile` (`conformance`) と `.github/workflows/ci.yml` (`conformance` ジョブ): 適合テストを実行し CI に組み込むため
    (T0411 の godot と rebase で合わせた)。
  - `docs/testing.md`・`docs/portability.md`・`AGENTS.md`: 適合テストの節とリンク、リポジトリ構成の 1 行。
- **足りない道具の案内:** `run.mjs` は最初に cc・Chrome・wasm-bindgen-cli (Cargo.lock の版)・GPU (`ugoe render` で小さなフィクスチャを描く) の順に調べ、
  足りないものと入れ方を出して終了コード 2 で止まる (`--only c` では C コンパイラだけ)。それぞれを外して案内が出ることを確かめた。
- **確かめていないこと:** CI の `conformance` ジョブ自体は動かしていない (push しないため。手元の同じ手順は通った)。
  wasm の WebGPU (`--backend webgpu`) は、ヘッドレスの Chrome (SwiftShader の Vulkan) で canvas の読み戻しが空になる・デバイスを失う・
  アダプタが取れなくなるのどれかになり、確かめられなかった (README の「確かめていないこと・制限」)。Windows・macOS では実行していない。
- **後続の候補:**
  - wasm に描画先を読み戻す API (テクスチャに描いてバッファにコピー) を足し、WebGPU でも画像を比べる。`WebPuppet` から不透明度・
    描画順の値などを取り出せるようにして、頂点以外も比べる。
  - Godot の `build.mjs --conformance` をこの計画と `ugoe-conformance compare` に乗せる (手順は README の「Godot」)。
    Godot だけにあるケース (マスクのモデル、値を変えた直後のフレーム) を `cases.json` に移す。
  - Unity・Unreal のバインディング (T0405・T0407) ができたら同じ手順で足す。
