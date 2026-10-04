---
id: T0694
title: エディタのパネルが読むモデルの JSON の解析を 1 回にする
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0671]
assignee:
---

# T0694: エディタのパネルが読むモデルの JSON の解析を 1 回にする

## 目的

T0671 (パネルのコードレビュー) の所見。パネルとインスペクタの節は、それぞれ `JSON.parse(doc.model.json)` でモデルを読み直している
(`DeformerProperties`・`DrawCloneSection`・`DepthSection`・`AvatarSection`・`PhysicsPanel`・`LightingPanel`・`SlotsPanel` (2 回)・
`GluesPanel`・`PosesPanel`・`VerifyPanel`、`MeshProperties` の `summarizeModel`、ツリーの `parseTreeModel`、`keyforms.ts` の `parseModel` は 1 つだけ覚える)。
コマンドを 1 つ適用するたびに、開いているパネルの数だけ同じ JSON を解析するので、頂点とキーフォームの多いモデル (JSON が数 MB) では
編集のたびの待ちが長くなる。同じ処理 (try / catch で null) も 10 か所に重複している。

## 受け入れ条件

- [ ] `EditorDocument` に、解析したモデル (`Model`) を読み直しのたびに 1 回だけ作って持たせ、パネルはそれを読むようにした
- [ ] ツリーとキーフォームの補助 (`parseTreeModel`・`parseModel`) も同じ解析の結果を使うか、使わない理由をメモに書いた
- [ ] 大きなモデル (例: 頂点 5 万・キーフォーム 9 セル) で、1 回の適用にかかる時間を前後で測り、メモに書いた
- [ ] `just ci` が通る

## 参考

- docs/tasks/T0671-review-editor-panels.md (所見の表)
- apps/editor/src/lib/document.svelte.ts

## メモ
