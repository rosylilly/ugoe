---
id: T0614
title: 推定したセルの由来 (推定の方法) を記録する
status: in-progress
milestone: M2
crates: [ugoe-model, ugoe-rigging, ugoe-editor-core]
depends_on: [T0611]
assignee: claude-worker-T0614
---

# T0614: 推定したセルの由来 (推定の方法) を記録する

## 目的

T0611 の自動の再推定は、格子の `inference` の方法 (加算合成など) で `derived` のセルを作り直す。
そのため、ミラーで作ったセルは、ミラー先の格子を編集したときに加算合成で上書きされてしまう。
セルごとに「どの方法で推定したか」を記録し、再推定ではその方法で作り直すようにする。

## 受け入れ条件

- [ ] `derived` のセルが、推定の方法 (加算合成・深度からの回転・ミラーとミラー元) を記録する。古いデータ (記録なし) は格子の `inference` の方法とみなす
- [ ] ミラーで作ったセルは、ミラー元の格子が変わったときにミラーで作り直され、ミラー先の編集では上書きされない
- [ ] 仕様 (`docs/spec/keyform-inference.md`) を更新する
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0611-inference-commands.md` のメモ
- `docs/spec/keyform-inference.md#セルの由来`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
