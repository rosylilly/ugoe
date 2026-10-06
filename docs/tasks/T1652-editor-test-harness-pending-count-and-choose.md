---
id: T1652
title: "エディタ: 部品のテストの FakeDocument に送っている数を持たせ、Select を選ぶ道具を harness にまとめる"
status: done
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

- [x] `FakeDocument` が送っている数を本物と同じ名前・意味 (`pending`) で持つ。今の約束の口は別の名前にし、使っているテスト (CurveGraph など) を直した
- [x] GluesPanel の部品のテストに、送っている間 (`apply` を待たせている間) はキー・軸・組を足すボタンが押せず、終わると押せることを足した
- [x] `Select` を選ぶ道具を `harness.svelte.ts` に 1 つ置き、各テストの自前の道具をそれに置き換えた
- [x] `pnpm --filter @ugoe/editor test`・`check` と `just ci` が通る

## 参考

- T1642 のメモ (分かったこと・起票の候補)
- apps/editor/src/lib (harness.svelte.ts・FakeDocument)、apps/editor/src/lib/panels/glues/GluesPanel.test.ts、
  apps/editor/src/lib/panels/inspector/inference/InferenceSection.test.ts、CurveGraph.test.ts

## メモ

- T0046 で起票した (元の候補: T1642 (FakeDocument の送っている数)・T1642 (Select を選ぶ道具をまとめる))。
- GluesPanel.test.ts を T1650・T1651 も触るので、その後にした。
- やったこと (T1652):
  - `FakeDocument.pending` を送っている数 (`$state(0)`) にした。`apply` は始めてから終わるまで、`send` は並べてから
    組み立てるまで数え、`apply` に渡すところで受け渡す (本物の `#enqueue` と同じく、1 つの送信を 1 と数える)。
    `apply` を待たせる約束は `gate` に名前を変え、使っていたテスト (CurveGraph・MotionEditor・SlotsPanel・GlueOverlay・
    DeformerGizmo・FormTool・MeshEditTool) を直した。
  - `harness.svelte.ts` に `choose(trigger, option)` を置いた (開くボタンを受け取り、下へ回って無ければ上へ移り、Enter で選び、
    一覧が消えるまで待つ)。CurveGraph の `chooseKind`・InferenceSection の `choose` はラベルから開くボタンを探して
    これを呼ぶだけにした (CurveGraph は種類の名前でなく表示の文字 "Step" を渡す)。GluesPanel のテストには自前の道具が無かった。
  - GluesPanel のテストに、`apply` を待たせている間はキー・軸・組を足すボタンが無効で押しても送らず、終わると押せることを足した
    (`busy` を常に偽にすると落ちることを確かめた)。
  - docs/testing.md (範囲の外の小さな変更) の部品のテストの節に `gate`・`pending`・`choose` を書いた。
- やらなかったこと: `apply.mockImplementationOnce` で `apply` を差し替えるテスト (InferenceSection) では `pending` は数えない
  (今それに頼るテストは無い)。
- 確認: `pnpm --filter @ugoe/editor test` (838 件)・`check`、`just ci` が通った。
