---
id: T0204
title: CLI でのスナップショット描画
status: todo
milestone: M1
crates: [ugoe-cli]
depends_on: [T0201]
assignee:
---

# T0204: CLI でのスナップショット描画

## 目的

`ugoe render <model> --param AngleX=10 -o out.png` で、ヘッドレスの wgpu で描画して PNG を書き出す。AI エージェントが見た目を確認する手段にもなる。

## 受け入れ条件

- [ ] フィクスチャのモデルを PNG に書き出せる
- [ ] パラメータを複数指定できる
- [ ] 出力の解像度と背景色を指定できる
- [ ] `just ci` が通る

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
