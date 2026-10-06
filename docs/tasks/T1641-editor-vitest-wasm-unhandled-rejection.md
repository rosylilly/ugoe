---
id: T1641
title: "エディタ: wasm の生成物があると、部品のテストで wasm の fetch が中断されて処理されない rejection になり Vitest が落ちる"
status: in-progress
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

- [ ] 生成物を置いた状態で、エディタの Vitest が落ちることを再現した (メモに手順)
- [ ] 部品のテストが本物の wasm を読みに行かないようにした (`loadUgoeWasm` を `vi.mock` する・テストの道具で偽物を渡す など)。読みに行く部品はどれかをメモに書いた
- [ ] 生成物があっても無くても `pnpm --filter @ugoe/editor test` が通る
- [ ] `just ci` が通る

## 参考

- T1633・T1634 のメモ、apps/editor/src/lib/panels/unreadableModel.test.ts、apps/editor/src/lib/viewport/wasm.ts

## メモ

- オーケストレーターが直接起票した (テストの揺れが 2 回出たため、候補の一覧を待たずに起票)。
