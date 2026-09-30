---
id: T0404
title: npm パッケージと Web Components
status: in-progress
milestone: M7
crates: [ugoe-wasm, bindings/web, docs]
depends_on: [T0402]
assignee: claude-worker-T0404
---

# T0404: npm パッケージと Web Components

## 目的

`@ugoe/runtime` (wasm + TypeScript の型) と `<ugoe-avatar>` Web Component を作る。

## 受け入れ条件

- [ ] `npm i @ugoe/runtime` 相当のローカルインストールで、モデルを読み込んで描画できるサンプルがある
- [ ] `<ugoe-avatar src="...">` でパラメータを属性や JS から設定できる
- [ ] WebGPU と WebGL2 の両方で動く
- [ ] ライセンスは MIT (利用者の決定、2026-10-01)。GPL-3.0-only の `packages/` とは分け、`bindings/web` などに置く。ライセンスの境界と置き場所を ADR 0025 に書き、ADR 0018/0019 との関係を示す。`just licenses` が npm の依存も検査するなら、MIT と両立しない依存を入れない
- [ ] `just ci` が通る

## 参考

- `docs/adr/0017-single-runtime-distribution.md`
- `docs/portability.md#配布`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
