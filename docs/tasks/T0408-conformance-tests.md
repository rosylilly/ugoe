---
id: T0408
title: 配布物の適合テスト
status: todo
milestone: M4
crates: [tests/conformance, CI]
depends_on: [T0401, T0404]
assignee:
---

# T0408: 配布物の適合テスト

## 目的

すべての配布物で、同じモデル・同じパラメータから同じ頂点と画像が得られることを検査する。

## 受け入れ条件

- [ ] 参照となる出力 (頂点と画像) を `ModelInstance` と `ugoe render` から生成する
- [ ] C ABI と wasm で比較するテストが CI で動く
- [ ] エンジン向けのバインディングが追加されたら同じテストに乗せる手順を書く
- [ ] `just ci` が通る

## 参考

- `docs/adr/0017-single-runtime-distribution.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
