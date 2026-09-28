---
id: T0505
title: 深度推定による振り向きリグの提案
status: todo
milestone: M5
crates: [ugoe-ai]
depends_on: [T0501, T0612]
assignee:
---

# T0505: 深度推定による振り向きリグの提案

## 目的

原画から深度マップを推定し、ワープデフォーマの制御点の `depth` と、`depth_rotation` による振り向きのキーフォームを提案する。

## 受け入れ条件

- [ ] 深度推定を `Provider` の裏に隠し、録画した応答でテストできる
- [ ] 意味ラベルで深度を補正する
- [ ] 提案が `validate` を通り、`ugoe render` で振り向きの画像を確認するテストがある
- [ ] `just ci` が通る

## 参考

- `docs/ai-integration.md#深度推定による振り向きリグ`
- `docs/spec/keyform-inference.md#深度からの回転`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
