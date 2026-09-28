---
id: T0106
title: 評価結果の変更フラグ
status: todo
milestone: M1
crates: [ugoe-runtime]
depends_on: [T0104]
assignee:
---

# T0106: 評価結果の変更フラグ

## 目的

描画可能要素ごとに、前フレームから頂点・不透明度・描画順・可視性が変わったかを示すフラグを出力する (エンジン統合での転送量を減らすため)。

## 受け入れ条件

- [ ] フラグの API とテストがある
- [ ] docs/spec/evaluation.md#8-出力 を更新する
- [ ] `just ci` が通る

## 参考

- `docs/spec/evaluation.md#8-出力`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
