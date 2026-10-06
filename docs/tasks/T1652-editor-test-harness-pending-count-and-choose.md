---
id: T1652
title: "エディタ: 部品のテストの FakeDocument に送っている数を持たせ、Select を選ぶ道具を harness にまとめる"
status: in-progress
milestone: M10
crates: [apps/editor]
depends_on: [T1651]
assignee: claude-worker-T1652
---

# T1652: エディタ: 部品のテストの FakeDocument に送っている数を持たせ、Select を選ぶ道具を harness にまとめる

## 目的

- 部品のテストの `FakeDocument.pending` は約束 (`apply` を待たせる) だが、本物の `EditorDocument.pending` は送っている数。
  GluesPanel の `busy` (`doc.pending > 0`) は偽物では常に偽なので、送っている間にボタンが押せないことを確かめられない (T1642)。
- 部品のテストで `Select` を選ぶ道具が、ファイルごとに書かれている (`CurveGraph.test.ts` の `chooseKind`・GluesPanel・InferenceSection の `choose`。T1642)。

## 受け入れ条件

- [ ] `FakeDocument` が送っている数を本物と同じ名前・意味 (`pending`) で持つ。今の約束の口は別の名前にし、使っているテスト (CurveGraph など) を直した
- [ ] GluesPanel の部品のテストに、送っている間 (`apply` を待たせている間) はキー・軸・組を足すボタンが押せず、終わると押せることを足した
- [ ] `Select` を選ぶ道具を `harness.svelte.ts` に 1 つ置き、各テストの自前の道具をそれに置き換えた
- [ ] `pnpm --filter @ugoe/editor test`・`check` と `just ci` が通る

## 参考

- T1642 のメモ (分かったこと・起票の候補)
- apps/editor/src/lib (harness.svelte.ts・FakeDocument)、apps/editor/src/lib/panels/glues/GluesPanel.test.ts、
  apps/editor/src/lib/panels/inspector/inference/InferenceSection.test.ts、CurveGraph.test.ts

## メモ

- T0046 で起票した (元の候補: T1642 (FakeDocument の送っている数)・T1642 (Select を選ぶ道具をまとめる))。
- GluesPanel.test.ts を T1650・T1651 も触るので、その後にした。
