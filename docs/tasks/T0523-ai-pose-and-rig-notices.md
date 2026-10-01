---
id: T0523
title: エディタと AI: 作り直しの知らせをプロジェクトに残す・差分のポーズの追従・ポーズの画面の操作
status: in-progress
milestone: M6
crates: [ugoe-ai, ugoe-editor-core, apps/editor, docs]
depends_on: [T0521]
assignee: claude-worker-T0523
---

# T0523: ポーズと自動リギングの知らせの残り

## 目的

T0521 の後続の候補をまとめる。

## 受け入れ条件

- [ ] 「リグを提案し直す」の知らせをプロジェクトに残し (今はセッションの間だけ)、インスペクタにも出す
- [ ] 絵の重なりで見つけた衣装の差分を、本体の差分に追従させる
- [ ] AI の分類に差分の手がかりを足す
- [ ] ポーズの画面に、ドラッグでの並べ替えと、表示中の要素の切り替えを足す
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0521-ai-module-pose-editing.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
