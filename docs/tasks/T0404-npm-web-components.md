---
id: T0404
title: npm パッケージと Web Components
status: todo
milestone: M7
crates: [ugoe-wasm, packages/]
depends_on: [T0402]
assignee:
---

# T0404: npm パッケージと Web Components

## 目的

`@ugoe/runtime` (wasm + TypeScript の型) と `<ugoe-avatar>` Web Component を作る。

## 受け入れ条件

- [ ] `npm i @ugoe/runtime` 相当のローカルインストールで、モデルを読み込んで描画できるサンプルがある
- [ ] `<ugoe-avatar src="...">` でパラメータを属性や JS から設定できる
- [ ] WebGPU と WebGL2 の両方で動く
- [ ] `just ci` が通る

## 参考

- `docs/adr/0017-single-runtime-distribution.md`
- `docs/portability.md#配布`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
