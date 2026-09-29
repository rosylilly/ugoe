---
id: T0114
title: ビルド時のテクスチャのアトラス化
status: todo
milestone: M6
crates: [ugoe-editor-core, ugoe-cli]
depends_on: [T0604]
assignee:
---

# T0114: ビルド時のテクスチャのアトラス化

## 目的

`.ugoe` の書き出し時に、描画順で隣り合う要素が同じテクスチャを使うように、テクスチャをアトラスにまとめる。

## 受け入れ条件

- [ ] アトラス化で UV を書き換え、見た目が変わらないことを `ugoe render` の画像比較で確認するテストがある
- [ ] 描画の状態の切り替え回数が減ることを `FrameStats` で確認する
- [ ] `ugoe build` (または書き出しのオプション) から使える
- [ ] `just ci` が通る

## 参考

- `docs/adr/0016-draw-batching.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
