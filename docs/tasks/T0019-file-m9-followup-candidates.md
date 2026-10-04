---
id: T0019
title: M9 の後続のタスクの「起票の候補」をまとめて起票する
status: todo
milestone: M9
crates: [docs]
depends_on: []
assignee:
---

# T0019: M9 の後続のタスクの「起票の候補」をまとめて起票する

## 目的

M9 の後続のタスクを並列で進めたワーカーは、新しいタスクを起票せずに「起票の候補」をメモに書いた (番号の衝突を避けるため)。
オーケストレーターが集めた候補の一覧を読み、重複をまとめ、既存のタスクに含まれるものは除いて、タスクとして起票する。

## 受け入れ条件

- [ ] オーケストレーターの一覧 (下の「参考」) と、M9 の done のタスクのメモの「起票の候補」を読んだ
- [ ] 同じことを言っている候補を 1 つのタスクにまとめ、既存のタスク (todo のもの) に含まれる候補はそのタスクのメモに書き足して起票しない
- [ ] 残りを docs/tasks/README.md の形でタスクにした (milestone は M9。受け入れ条件は検証できる形。depends_on を正しく。`crates:` は狭く)
- [ ] 公開 API の変更や仕様の判断が要るものは、タスクの本文に「判断が要る」と書いた
- [ ] 番号: エディタの帯 (T06xx) は T0699 まで使い切ったので、2 つ目の範囲として「T1 + 分野の 1 桁 + 2 桁」(エディタなら T1600〜) を使う規則を docs/tasks/README.md の表に足した。ほかの帯は空いている番号を使う
- [ ] 起票したタスクの一覧 (ID・題名・元の候補) をメモに書いた
- [ ] `node scripts/tasks.mjs` が通る (未知の依存が無い)
- [ ] `just ci` が通る

## 参考

- オーケストレーターの集計: /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/026b491e-8c79-47ed-ac14-04f23c5addc0/scratchpad/run2/candidates.md
- docs/tasks/README.md、docs/workflow/multi-agent.md#タスクの書き方-オーケストレーター向け

## メモ
