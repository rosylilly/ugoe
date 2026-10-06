---
id: T1641
title: "エディタ: wasm の生成物があると、部品のテストで wasm の fetch が中断されて処理されない rejection になり Vitest が落ちる"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1641
---

# T1641: エディタ: wasm の生成物があると、部品のテストで wasm の fetch が中断されて処理されない rejection になり Vitest が落ちる

## 目的

`apps/editor/src/lib/viewport/generated/` に wasm の生成物 (`just wasm` や `just e2e` が作る。git の対象外) があると、エディタの Vitest で
`panels/unreadableModel.test.ts` の後に、`TimelinePanel` などが読みに行った wasm の fetch がテストの片付けで中断され、処理されない rejection が 2 件残る。
テストはすべて通るのに Vitest が失敗になり、`just e2e` の後に流した `just ci` が落ちる (T1633・T1634 の作業中に 2 回出た。候補)。

## 受け入れ条件

- [x] 生成物を置いた状態で、エディタの Vitest が落ちることを再現した (メモに手順)
- [x] 部品のテストが本物の wasm を読みに行かないようにした (`loadUgoeWasm` を `vi.mock` する・テストの道具で偽物を渡す など)。読みに行く部品はどれかをメモに書いた
- [x] 生成物があっても無くても `pnpm --filter @ugoe/editor test` が通る
- [x] `just ci` が通る

## 参考

- T1633・T1634 のメモ、apps/editor/src/lib/panels/unreadableModel.test.ts、apps/editor/src/lib/viewport/wasm.ts

## メモ

- オーケストレーターが直接起票した (テストの揺れが 2 回出たため、候補の一覧を待たずに起票)。
- 再現 (claude-worker-T1641): worktree で `just wasm` を流して `apps/editor/src/lib/viewport/generated/` を作り、`pnpm --filter @ugoe/editor test` を
  3 回流すと 3 回とも、84 ファイル・696 件が通ったうえで `panels/unreadableModel.test.ts` 由来の Unhandled Rejection
  (happy-dom の `Fetch.onAsyncTaskManagerAbort` の `AbortError`) が 2 件出て終了コード 1 になった。
- 読みに行く部品: `loadUgoeWasm` を呼ぶのは `panels/timeline/timeline.svelte.ts` (`TimelineState` のコンストラクタ。`TimelinePanel` が作る) と
  `viewport/controller.svelte.ts` (`ViewportController` の既定の `loadWasm`)。テストで本物を読んでいたのは `panels/unreadableModel.test.ts` の
  `TimelinePanel` だけ (ほかのテストはファイルの中で `vi.mock` していた。`ViewportController` はテストでは偽物か `loadWasm` を渡している)。
- 直し方: `src/test-setup.ts` で `viewport/wasm` を `vi.mock` し、`loadUgoeWasm` を null を返す偽物にした (新しい部品のテストでも漏れないように、
  ファイルごとではなくまとめて)。ファイルの中の `vi.mock` はこれを上書きする (`timeline-state.test.ts` の偽の `MotionSampler` はそのまま効く)。
  前提を確かめるテスト `src/test-setup.test.ts` (偽物に置き換わっていること) を足し、mock を外すと失敗することを確かめた。docs/testing.md の
  「エディタの部品のテスト」に書いた。
- 生成物がある状態とない状態の両方で `pnpm --filter @ugoe/editor test` が通る (85 ファイル・697 件、rejection なし)。`just ci` は生成物がある状態で通った。
- やらなかったこと: `CurveGraph.test.ts`・`MotionEditor.test.ts` の null を返す `vi.mock` は不要になったが、T1640 が並行して apps/editor を触っているので消していない。
