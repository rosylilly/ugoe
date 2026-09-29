---
id: T0210
title: ライティングをホストにつなぐ
status: done
milestone: M4
crates: [ugoe, ugoe-wasm, ugoe-headless, ugoe-cli, docs]
depends_on: [T0209]
assignee: claude-worker-T0210
---

# T0210: ライティングをホストにつなぐ

## 目的

T0209 で参照レンダラにライティング (`Lighting`・`set_normal_map`) を足したが、`Puppet`・wasm・ヘッドレス描画はまだバンドルの法線マップを登録せず、光源も渡さない。
ホストからライティングを使えるようにする。

## 受け入れ条件

- [x] バンドルを読んだホストが、`Bundle::normal_maps` を `Renderer::set_normal_map` で登録する (`ugoe` の Puppet まわり・`ugoe-headless`・`ugoe-wasm`)
- [x] wasm と ugoe-headless から光源 (`Lighting`) を渡せる。`ugoe render` に光源の指定 (例: `--ambient`・`--light`) を足す
- [x] 法線マップ付きのバンドルを `ugoe render` で描き、光源の向きで陰影が変わる GPU テストがある
- [x] `just ci`・`just wasm-check`・`just test-gpu` が通る

## 参考

- `docs/tasks/T0209-lighting.md` のメモ
- `docs/spec/rendering.md#ライティング`

## メモ

- エディタとビューアのプレビューでの光源の UI は、別のタスクにする。

- やったこと:
  - `ugoe` (SDK): `ugoe::lighting` を足した (`bundle_normal_maps`・`uses_lighting`・`Puppet::uses_lighting`)。`Puppet` はレンダラを
    持たないので、法線マップのデコードと登録はホストが行う。T0121 と衝突しないよう新しいファイルにまとめ、`lib.rs` は `pub mod` の 1 行だけ。
  - `ugoe-headless`: `Scene::normal_maps`・`resolve_normal_maps` (PNG をアルファを掛けずにデコード。KTX2 は警告して使わない)・
    `RenderOptions::lighting` (`None` なら `set_lighting` を呼ばない)・`Prepared::shading`。`Gpu::render` が `set_normal_map` と `set_lighting` を呼ぶ。
  - `ugoe render`: `--ambient <V|R,G,B>` と `--light directional:DX,DY,DZ[:COLOR]` / `--light point:X,Y,Z[:COLOR[:RANGE]]` (複数可)。
    どちらも無ければ光源を渡さない。書式は `crates/ugoe-cli/README.md` と `docs/spec/rendering.md#ホストでのライティング`。
    `ugoe-cli` の `Input` は `ugoe_headless::Scene` に置き換えた。
  - `ugoe-wasm`: `load_bundle` が法線マップを登録し (前のモデルの分は `remove_normal_map` で外す)、`set_lighting_json(json | undefined)` と
    `uses_lighting()` を足した (`src/web/lighting.rs`。JSON の読み取りはネイティブでテストできる `src/lighting.rs`)。光源はモデルを読み直しても保つ。
  - GPU テスト `crates/ugoe-cli/tests/lighting_render_gpu.rs`: ふくらみの法線マップ付きのバンドルをテストの中で組み立て、光源の向き
    (左・右・上・点光源) で明るい側が変わること、光源を指定しなければ法線マップの無いバンドルとバイト単位で同じ PNG になることを確かめる。
  - 出力が変わらないことの確認: main のバイナリと、全フィクスチャ (11 個) とテストのバンドルを光源なしで描いた PNG がバイト単位で一致。
- やらなかったこと: `ugoe-ffi` (crates: の範囲外)、MCP サーバの `render_png` の光源 (法線マップは headless を通して登録されるが光源は渡さない)、
  エディタ・ビューアの光源の UI。
- 後続の候補: ffi から光源と法線マップを渡す API、ビューア (と エディタのプレビュー) の光源の UI (`set_lighting_json`・`uses_lighting` を使う)、
  MCP の `render_png` に光源の引数、KTX2 の法線マップのデコード。
