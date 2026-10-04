---
id: T0698
title: エディタのモデルの JSON を 1 回だけ解析して共有する
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0672, T0671]
assignee: claude-worker-T0698
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

- [x] `EditorDocument` が解析済みのモデルを持ち、ビューポート・プレビュー・パネルの `JSON.parse(doc.model.json)` をそれに置き換える
- [x] 解析できないモデルで部品が止まらない (テスト)
- [x] ツリーとキーフォームの補助 (`parseTreeModel`・`parseModel`) も同じ解析の結果を使うか、使わない理由をメモに書いた (T0694)
- [x] 大きなモデル (例: 頂点 5 万・キーフォーム 9 セル) で、1 回の適用にかかる時間を前後で測り、メモに書いた (T0694)
- [x] `just ci` が通る

## 参考

- T0672 のメモ (所見の表)、T0671 のメモ (所見の表)、まとめる前の T0694
- apps/editor/src/lib/document.svelte.ts

## メモ

- **やったこと:** `EditorDocument.parsedModel` (`$derived`) が `model.json` を解析して持つ。最後に解析した JSON を覚え、読み直しで
  JSON が同じ文字列なら解析し直さずに同じオブジェクトを返す (T0671 の「読み直しのたびに作り直すオブジェクトに `$effect` が反応する」を
  モデルについては起こさない。`$derived` の連鎖での `===` の比較は反応の文脈の外 (テスト) では効かなかったので、明示的に覚える)。
  読めない JSON (と JSON のオブジェクトでないもの) は null (`parseModelJson`)。`refresh` はこれを使い、null なら従来どおり `error` に出す。
- 置き換えた所: ビューポート (`ViewportScene`・`FormTool`・`DeformerGizmo`・`DepthOverlay`・`OverflowOverlay`・`GlueOverlay`・`LightingOverlay`)、
  プレビュー (`PreviewPanel`・`DifferenceOverlay`・`PreviewMode.#readPose`)、パネル (`InspectorPanel` の見出し・`DeformerProperties`・
  `DrawCloneSection` (作った後の読み直しも)・`DepthSection`・`AvatarSection`・`MeshProperties`・`KeyformInspector`・`InferenceSection`・
  `ParametersPanel`・`PhysicsPanel`・`LightingPanel`・`SlotsPanel` (2 回とも)・`GluesPanel`・`PosesPanel`・`VerifyPanel`・`TreePanel`)、`App.svelte`。
  `PreviewPanel` と `InspectorPanel` の見出し (`parseModel`) は try/catch が無く、読めない JSON で例外になっていた。
- **ツリーとキーフォームの補助 (T0694):** どちらも同じ解析の結果を使う。`parseTreeModel(json)` は `treeModel(model)` (解析済みのモデルから
  ツリーの部分を取り出す。欠けた配列は空)、`keyforms.ts` の `parseModel(json)` (直近 1 つを覚える自前の解析) は消し、`modelData(model)`
  (同じオブジェクトを `ModelData` の型で見る) にした。`summarizeModel` も解析済みのモデルを受け取る。
- **テスト:** `document.test.ts` (同じ JSON の読み直しで同じオブジェクト・変われば作り直す・読めない JSON は null で `error`・
  `parseModelJson`)、`scene.test.ts` (読めない JSON でビューポートの要約が空で止まらない)、`build.test.ts` (`treeModel` の欠けた配列)。
  エディタには Svelte の部品を描くテストの道具が無いので、部品そのものは描いて確かめていない (部品は `parsedModel` が null のときに
  何も出さない形にそろった: 前の try/catch で null の扱いと同じ)。
- **時間の計測 (T0694):** 頂点 5 万・キーフォーム 9 セル (3×3) のメッシュ 1 つのモデルの JSON (12.1 MB) を作り、Node 26 (V8) で
  `JSON.parse` 1 回が 62 ms。メッシュを選んでインスペクタを開いた既定の配置では、前は 1 回の適用で 8 回 (`refresh`・`ViewportScene`・
  `keyforms.ts` の共有の 1 回・ツリー・`MeshProperties`・`DrawCloneSection`・`DeformerProperties`・`DepthSection`) 解析していて約 500 ms、
  後は 1 回で約 62 ms (右の列のタブ・オーバーレイ・プレビューを開いていればそれぞれ前は +62 ms、後は増えない)。
  計測の台本は解析の部分だけで、実機の WebView (WebKitGTK・WebView2) での適用全体 (IPC の転送・WebPuppet の `load_json`。どちらも変わらない)
  は測っていない。JavaScriptCore の `JSON.parse` の速さは V8 と違うので、数字は目安。
- 解析したモデルは部品の間で共有するので、部品は書き換えてはならない (docs/editor.md に書いた。今の部品に書き換える所が無いことは
  配列の破壊的な操作と代入を検索して確かめた)。
- 残した `JSON.parse`: `get_model_json` 以外の JSON (`timeline` のサンプル・`dressUp.ts`・`loadWarnings.ts`・`preview.ts` の合成したポーズ・
  `DifferenceOverlay` の物理の設定・AI の提案のプレビュー (`proposals.svelte.ts`。ドキュメントのモデルではない))。
- **起票の候補:** エディタに Svelte の部品を描くテストの道具 (`@testing-library/svelte` など) を入れ、「読めないモデルで部品が止まらない」を
  部品の単位で確かめる。

- main (T0693 の編集の列) に rebase し、衝突 (`document.test.ts`・`DepthSection`・`PhysicsPanel` の import) を解いた。T0693 の `send(build)` が
  組み立てに読む `model` も `doc.parsedModel` になる (読み直しの後の `tick()` で新しいものが行き渡る)。rebase の後に `just ci` が通った。
- T0699 (ビューポートのツールの部品のテスト) の後に rebase し、偽の document (`viewport/tools/test/harness.svelte.ts` の `FakeDocument`) に
  `parsedModel` を足した。あわせて、main で落ちていた `ToolOverlay.test.ts` の「Delete で選んだ頂点を消す」を直した: T0693 で
  `MeshEditTool` が `apply` の戻り値で成否を見るようになったが、`FakeDocument.apply` は null (失敗) を返していた。本物と同じく
  成功の報告 (`{ warnings: [], mirrored: [] }`) を返すようにした。これで部品を描くテストの道具ができたので、上の「起票の候補」の
  道具は入った (読めないモデルで各部品が止まらないことを部品の単位で確かめるテストは、まだ書いていない)。rebase の後に `just ci` が通った。
