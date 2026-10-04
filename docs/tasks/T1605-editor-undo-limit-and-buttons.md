---
id: T1605
title: "エディタ: Undo の上限の設定と、戻れない・進めないときの Undo/Redo のボタン"
status: todo
milestone: M9
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee:
---

# T1605: エディタ: Undo の上限の設定と、戻れない・進めないときの Undo/Redo のボタン

## 目的

T0680 で Undo の履歴に上限 (既定 100) を設け、`Document::set_undo_limit` を用意したが、エディタの設定には出していない
(大きなモデルでは 1 状態が数 MB になり、上限を下げたい利用者がいる)。また、画面の Undo / Redo のボタンは常に押せ、戻れないときは何もしない。

## 受け入れ条件

- [ ] `Document` に `can_undo`・`can_redo` (と必要なら履歴の数) があり、テストがある
- [ ] 編集・Undo・Redo の後に、その状態が画面へ届き、ボタン (とメニューの項目) が押せなくなる
- [ ] エディタの設定に Undo の上限があり、変えると `set_undo_limit` が呼ばれ、次の起動でも覚えている (範囲と既定値を決めた)
- [ ] `apps/editor/src/lib/ipc.ts` と `docs/editor.md` を更新した
- [ ] `just ci` が通る

## 参考

- T0680 のメモ (起票の候補)
- crates/ugoe-editor-core/src/document.rs (`set_undo_limit`)

## メモ

- T0019 で起票した (元の候補: T0680 の 2 つ)。
