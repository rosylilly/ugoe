---
id: T0705
title: ビューア: ライティングの光源の操作
status: in-progress
milestone: M4
crates: [apps/viewer, docs]
depends_on: [T0210]
assignee: claude-worker-T0705
---

# T0705: ビューア: ライティングの光源の操作

## 目的

利用者の指摘。wasm には光源の口 (`set_lighting_json`) があるが、ビューアで操作できない。

## 受け入れ条件

- [ ] ライティングを使うモデル (`uses_lighting`) を開いたら、光源のパネル (環境光・平行光源・点光源の向き・色・強さ。既定の光源の組) を出す。設定をビューアに残す
- [ ] ヘッドレスのブラウザで確かめる。ビューアの README とマニュアルを更新する
- [ ] `just ci`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0210-lighting-hosts.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
