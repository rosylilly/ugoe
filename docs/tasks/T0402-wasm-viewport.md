---
id: T0402
title: wasm の WebPuppet とエディタのビューポート
status: todo
milestone: M2
crates: [ugoe-wasm, apps/editor (src/ のみ)]
depends_on: [T0201]
assignee:
---

# T0402: wasm の WebPuppet とエディタのビューポート

## 目的

`ugoe-wasm` に `WebPuppet` (モデルの読み込み、パラメータの設定、canvas への描画) を作り、エディタのビューポートで表示する。

## 受け入れ条件

- [ ] WebGPU と WebGL2 のどちらでも描画できる
- [ ] パラメータスライダーの操作が IPC を経由せずにビューポートに反映される
- [ ] wasm のビルド手順を justfile に追加する
- [ ] `just ci` が通る

## 参考

- `docs/editor.md#ビューポート`
- `docs/adr/0003-editor-viewport-in-webview.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
