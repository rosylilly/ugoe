---
id: T0111
title: 着せ替え: スロットとモジュールの合成
status: in-progress
milestone: M4
crates: [ugoe-model, ugoe-runtime]
depends_on: [T0109]
assignee: claude-worker-T0111
---

# T0111: 着せ替え: スロットとモジュールの合成

## 目的

`Model::slots` とモジュールのデータを定義し、本体とモジュールの組を合成して 1 つのモデルを作る。

## 受け入れ条件

- [ ] スロットとモジュールの型を定義し、`validate` が参照を検査する
- [ ] `compose(base, modules)` が ID の付け替え・取り付け・描画順の写像・`replaces` を行い、結果が `validate` を通る
- [ ] 組み替えでパラメータの値を引き継ぐ
- [ ] 中規模のモデルで合成とコンパイルにかかる時間を計測し、結果をメモに書く
- [ ] フィクスチャとテストがある
- [ ] `just ci` が通る

## 参考

- `docs/adr/0014-composable-modules.md`
- `docs/spec/modules.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
