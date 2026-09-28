---
id: T0108
title: キーフォーム推定のためのデータ定義
status: in-progress
milestone: M2
crates: [ugoe-model, ugoe-format (テストの追従のみ)]
depends_on: []
assignee: claude-worker-T0108
---

# T0108: キーフォーム推定のためのデータ定義

## 目的

キーフォームのセルの由来 (`authored` / `derived`)、推定の設定、ミラーの対応 (`mirror`)、頂点・制御点の `depth` を `ugoe-model` に追加する (インターフェースを先に入れるタスク)。

## 受け入れ条件

- [ ] `KeyformGrid::origins` (セルごとの由来、既定はすべて `authored`) と推定の設定を追加し、serde で往復できる
- [ ] `Mesh::mirror` / `Deformer::mirror` と、頂点・制御点の `depth` (オーサリング用、既定なし) を追加する
- [ ] `validate` が由来の長さと `mirror` の参照を検査する
- [ ] 古い JSON (新しいフィールドなし) が読める
- [ ] `docs/spec/model.md` の「予定されているデータ」と `docs/spec/keyform-inference.md` を実装に合わせて更新する
- [ ] `just ci` が通る

## 参考

- `docs/adr/0010-keyform-inference.md`
- `docs/spec/keyform-inference.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
