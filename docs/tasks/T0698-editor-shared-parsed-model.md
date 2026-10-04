---
id: T0698
title: エディタのモデルの JSON を 1 回だけ解析して共有する
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0672]
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

## 受け入れ条件

- [ ] `EditorDocument` が解析済みのモデルを持ち、ビューポート・プレビュー・パネルの `JSON.parse(doc.model.json)` をそれに置き換える
- [ ] 解析できないモデルで部品が止まらない (テスト)
- [ ] `just ci` が通る

## 参考

- T0672 のメモ (所見の表)

## メモ
