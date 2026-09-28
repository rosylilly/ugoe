---
id: T0201
title: 参照レンダラ: パイプラインと描画
status: in-progress
milestone: M1
crates: [ugoe-render]
depends_on: []
assignee: claude-worker-T0201
---

# T0201: 参照レンダラ: パイプラインと描画

## 目的

`Renderer` の `set_texture` / `encode` を実装し、マスクなしの描画可能要素を 3 種類の合成モードで描画する。

## 受け入れ条件

- [ ] パイプラインを (合成モード × カリング) の組み合わせでキャッシュする
- [ ] 乗算色・スクリーン色・不透明度が仕様どおりに反映される
- [ ] WebGL2 で使えない機能を使っていない (`just wasm-check` が通る)
- [ ] `gpu-tests` feature で、オフスクリーン描画して画素を検証するテストがある
- [ ] `just ci` が通る

## 参考

- `docs/spec/rendering.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
