---
id: T1639
title: "エディタ: 提案の一覧の分類の直しの行に、残すラベルと左右を出す"
status: in-progress
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0676]
assignee: claude-worker-T1639
---

# T1639: エディタ: 提案の一覧の分類の直しの行に、残すラベルと左右を出す

## 目的

T1631 で、エージェントの提案に添えた分類の直し (`set_layer_correction`) を提案の一覧の 1 つの行にまとめた。行はレイヤーの数を言うだけで、
どのレイヤーにどのラベルと左右を残すのかが一覧では分からない (T1631 の候補)。

## 受け入れ条件

- [ ] バックエンド (`apps/editor/src-tauri`) の `ProposalCommand` に分類の直しの要約 (レイヤーの名前・ラベル・左右) があり、IPC の型 (`apps/editor/src/lib/ipc.ts`) を合わせた
- [ ] 提案の一覧の分類の直しの行を開くと、レイヤーごとに `Face / Eye R → eye_white (right)` の形で出す (長い一覧の扱いをメモに書く)。部品のテストがある
- [ ] src-tauri の要約の単体テストがある
- [ ] docs/editor.md (提案の一覧) を直した
- [ ] `just ci` が通る

## 参考

- T1631 のメモ (やったこと・起票の候補)
- apps/editor/src-tauri/src (`ProposalCommand`)、apps/editor/src/lib/ai (提案の一覧)

## メモ

- T0041 で起票した (元の候補: T1631 (分類の直しの要約))。T0676 (作業中) が AI のパネルを触るので、その後に着手する。
