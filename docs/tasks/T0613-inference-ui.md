---
id: T0613
title: エディタ: 推定の操作と由来の表示
status: todo
milestone: M2
crates: [apps/editor]
depends_on: [T0611, T0602, T0608]
assignee:
---

# T0613: エディタ: 推定の操作と由来の表示

## 目的

パラメータのキーの一覧で、セルの由来 (`authored` / `derived`) を表示し、推定・ミラー・推定のやり直しを操作できるようにする。

## 受け入れ条件

- [ ] 操作はすべて `Command` を送ることで行う
- [ ] `derived` のセルが見た目で区別できる
- [ ] 自動のミラーの作り直しが失敗した (ミラー元の頂点が許容距離を超えて動いた) ことを利用者に知らせる (T0614 のメモ)
- [ ] `just web-check` が通る
- [ ] `just ci` が通る

## 参考

- `docs/spec/keyform-inference.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
