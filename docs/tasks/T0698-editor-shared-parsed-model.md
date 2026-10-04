---
id: T0698
title: エディタのモデルの JSON を 1 回だけ解析して共有する
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0672, T0671]
assignee:
---

# T0698: エディタのモデルの JSON を 1 回だけ解析して共有する

## 目的

編集のたびに `EditorDocument.model.json` (モデル全体の JSON) が置き換わり、それを読む部品がそれぞれ `JSON.parse` している。
ビューポートとプレビューだけでも、`ViewportScene` (`summarizeModel`)・`FormTool`・`DeformerGizmo` (`parseModel`)・
`DepthOverlay`・`OverflowOverlay`・`GlueOverlay`・`LightingOverlay`・`DifferenceOverlay` (`summarizeModel`)・`PreviewPanel`・
`PreviewMode.#readPose` と 10 か所ほどあり、頂点の多いモデルでは 1 回の編集 (ドラッグを離すたび) で同じ解析を繰り返す。
パネル (インスペクタなど) にも同じ形がある。

`EditorDocument` に解析済みのモデル (`$derived`) を 1 つ持たせ、部品はそれを読む。解析できないとき (`null`) の扱いを揃える
(今は try/catch するところと、しないところ (`PreviewPanel`) がある)。

T0671 のレビューの同じ所見 (T0694) をこのタスクにまとめた (T0008)。パネルとインスペクタの節でも、`DeformerProperties`・`DrawCloneSection`・
`DepthSection`・`AvatarSection`・`PhysicsPanel`・`LightingPanel`・`SlotsPanel` (2 回)・`GluesPanel`・`PosesPanel`・`VerifyPanel`、`MeshProperties` の
`summarizeModel`、ツリーの `parseTreeModel`、`keyforms.ts` の `parseModel` がそれぞれ解析している (try / catch で null の同じ処理が 10 か所)。

## 受け入れ条件

- [ ] `EditorDocument` が解析済みのモデルを持ち、ビューポート・プレビュー・パネルの `JSON.parse(doc.model.json)` をそれに置き換える
- [ ] 解析できないモデルで部品が止まらない (テスト)
- [ ] ツリーとキーフォームの補助 (`parseTreeModel`・`parseModel`) も同じ解析の結果を使うか、使わない理由をメモに書いた (T0694)
- [ ] 大きなモデル (例: 頂点 5 万・キーフォーム 9 セル) で、1 回の適用にかかる時間を前後で測り、メモに書いた (T0694)
- [ ] `just ci` が通る

## 参考

- T0672 のメモ (所見の表)、T0671 のメモ (所見の表)、まとめる前の T0694
- apps/editor/src/lib/document.svelte.ts

## メモ
