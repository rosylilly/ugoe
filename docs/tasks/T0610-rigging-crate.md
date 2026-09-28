---
id: T0610
title: ugoe-rigging クレートと加算合成
status: todo
milestone: M2
crates: [ugoe-rigging (新規), scripts/check-layers.mjs, commitlint.config.mjs, docs/architecture.md]
depends_on: [T0108]
assignee:
---

# T0610: ugoe-rigging クレートと加算合成

## 目的

ツール層のクレート `ugoe-rigging` を作り、キーフォームの加算合成 (交互作用の項を含む) を実装する。

## 受け入れ条件

- [ ] クレートを追加し、依存方向の表 (`scripts/check-layers.mjs`) とコミットの scope (`rigging`) に登録する
- [ ] `docs/spec/keyform-inference.md#加算合成` のすべての式にテストがある (1 軸の補間・外挿、交互作用の項の伝播、クランプ)
- [ ] `authored` のセルを上書きしない
- [ ] 中立のキーがない軸はエラーになる
- [ ] `just ci` が通る

## 参考

- `docs/adr/0010-keyform-inference.md`
- `docs/spec/keyform-inference.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
