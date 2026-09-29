---
id: T0210
title: ライティングをホストにつなぐ
status: todo
milestone: M4
crates: [ugoe, ugoe-wasm, ugoe-headless, ugoe-cli, docs]
depends_on: [T0209]
assignee:
---

# T0210: ライティングをホストにつなぐ

## 目的

T0209 で参照レンダラにライティング (`Lighting`・`set_normal_map`) を足したが、`Puppet`・wasm・ヘッドレス描画はまだバンドルの法線マップを登録せず、光源も渡さない。
ホストからライティングを使えるようにする。

## 受け入れ条件

- [ ] バンドルを読んだホストが、`Bundle::normal_maps` を `Renderer::set_normal_map` で登録する (`ugoe` の Puppet まわり・`ugoe-headless`・`ugoe-wasm`)
- [ ] wasm と ugoe-headless から光源 (`Lighting`) を渡せる。`ugoe render` に光源の指定 (例: `--ambient`・`--light`) を足す
- [ ] 法線マップ付きのバンドルを `ugoe render` で描き、光源の向きで陰影が変わる GPU テストがある
- [ ] `just ci`・`just wasm-check`・`just test-gpu` が通る

## 参考

- `docs/tasks/T0209-lighting.md` のメモ
- `docs/spec/rendering.md#ライティング`

## メモ

- エディタとビューアのプレビューでの光源の UI は、別のタスクにする。

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
