---
id: T0501
title: PSD 取り込み
status: todo
milestone: M2
crates: [ugoe-psd]
depends_on: []
assignee:
---

# T0501: PSD 取り込み

## 目的

PSD から `LayerTree` を作る。外部クレートを使うか自前で書くかを決める。

## 受け入れ条件

- [ ] グループ・表示状態・不透明度・位置が正しく読める
- [ ] テスト用の小さな PSD を `assets/fixtures/` に置き、出所を README に書く
- [ ] 未対応の機能を含む PSD では警告を返す
- [ ] `just ci` が通る

## 参考

- `docs/ai-integration.md#psd-取り込み`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
