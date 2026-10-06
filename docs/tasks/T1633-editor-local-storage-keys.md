---
id: T1633
title: "エディタ: localStorage のキーの名前を ugoe.editor.* にそろえる"
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0675]
assignee:
---

# T1633: エディタ: localStorage のキーの名前を ugoe.editor.* にそろえる

## 目的

エディタが localStorage に覚える値のキーの形がそろっていない (`ugoe.editor.*`・`ugoe.timeline`・`ugoe-editor.locale`・`ugoe-editor.theme`・`ugoe-editor.mcpb` など)。
T0670 は「変えると利用者の覚えた値が消える」として見送ったが、リリース前なので覚えた値の移し替えは要らない (利用者の方針)。T0673 で区切りのキーは `ugoe.editor.*` にそろえた (T0673 の候補)。

## 受け入れ条件

- [ ] エディタの localStorage のキーがすべて `ugoe.editor.<名前>` の形で、1 か所 (定数の一覧) から使う。古いキーを読む処理や移し替えは入れない
- [ ] キーの一覧とその形が単体テストで確かめられる (同じキーを 2 つの用途で使わない)
- [ ] docs/editor.md (覚える値) を直した。ビューア・スタジオのキーの形も調べ、そろっていなければメモに書いた (直すのは範囲の外)
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0673 のメモ (起票の候補)、T0670 のメモ (見送りの表)
- apps/editor/src/lib (`locale.ts`・`layout.ts`・`externalChange.ts`・`panels/timeline/timeline.ts`・`agents/ConnectAgents.svelte`)、apps/editor/src/main.ts

## メモ

- T0041 で起票した (元の候補: T0673 (localStorage のキーの名前))。T0675 (レビュー中) がタイムラインを触るので、その後に着手する。
