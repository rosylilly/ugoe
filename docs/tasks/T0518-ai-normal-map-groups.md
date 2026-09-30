---
id: T0518
title: AI: 法線マップの生成で、部位のまとまりごとにふくらませる
status: todo
milestone: M6
crates: [ugoe-ai, ugoe-rigging, apps/editor, docs]
depends_on: [T0707]
assignee:
---

# T0518: AI: 法線マップの生成で、部位のまとまりごとにふくらませる

## 目的

T0707 の後続の候補。T0707 の見本 (`ugoe sample avatar --lit`) は、部位をまとまり (顔と目口鼻眉・首と体と腕・髪・耳) に分け、まとまりごとに 1 つの形としてふくらませることで、
目が顔の上で平らに浮いたり、首と肩に段差が出たりするのを防いだ (`crates/ugoe-cli/src/sample/lit.rs`)。
エディタの Generate と `ugoe_ai::normal` の提案は、今もパーツごとにふくらませている。

## 受け入れ条件

- [ ] 意味ラベル (T0503) からまとまりを決め、まとまりごとにふくらませる生成を `ugoe-ai` (または `ugoe-rigging`) に置く。T0707 の見本の生成もそれを使うかを決める (使うなら見本のファイルが変わらないことを確かめる)
- [ ] 意味ラベルごとの既定の強さと合わせる。形の外に法線を広げて輪郭に線が出ないようにする
- [ ] エディタの法線マップの生成 (Lighting タブ) と AI の提案がこの方法を使う
- [ ] 描いた絵を目で確かめる。`just ci`・`just test-gpu`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0707-lit-sample-avatar.md`・`docs/tasks/T0506-normal-map-gen.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
