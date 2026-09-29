---
id: T0112
title: 法線マップとライティングのデータ
status: todo
milestone: M4
crates: [ugoe-model, ugoe-format]
depends_on: [T0109, T0111]
assignee:
---

# T0112: 法線マップとライティングのデータ

## 目的

`TextureRef::normal_map`、`Model::lighting`、`Mesh::lit` と、バンドルの `NRML` チャンクを追加する。

## 受け入れ条件

- [ ] 型とシリアライズ、`NRML` チャンクの読み書きがある
- [ ] 機能 `lighting.normal_map` を宣言する
- [ ] 仕様書の「実装待ち」を更新する
- [ ] `just ci` が通る

## 参考

- `docs/adr/0015-lighting.md`
- `docs/spec/rendering.md#ライティング`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
