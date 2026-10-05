---
id: T1613
title: "エディタ: 光源のドラッグと、読めないモデルで部品が止まらないことを部品の単位でテストする"
status: done
milestone: M9
crates: [apps/editor]
depends_on: [T1612]
assignee: claude-worker-T1613
---

# T1613: エディタ: 光源のドラッグと、読めないモデルで部品が止まらないことを部品の単位でテストする

## 目的

T0699 でビューポートのツールの部品を描くテストの道具 (`viewport/tools/test/harness.svelte.ts` の `FakeDocument`) ができた。

- `LightingOverlay` (`viewport/tools/lighting/LightingOverlay.svelte`) の光源のドラッグのテストが無い (T0699 の候補)。
- モデルの JSON が読めないとき (`doc.parsedModel` が無い) にパネルと重ねものが止まらないことを、部品ごとに確かめるテストが無い (T0698 の候補)。

## 受け入れ条件

- [x] `LightingOverlay` のドラッグで、離したときに 1 つのコマンドが送られ、ドラッグ中は送らないテストがある
- [x] 読めないモデルで、主なパネルと重ねもの (どれを対象にしたかをメモに) が例外を出さずに描けるテストがある
- [x] `just ci` が通る

## 参考

- T0699・T0698 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0699 の `LightingOverlay`、T0698 の部品の単位のテスト)。部品を触る T1612 の後に着手する。
- T0699 の候補「T0698 が `doc.model` の読み方を変えたら `FakeDocument` にも足す」は T0698 で入った (`parsedModel`) ので起票しない。
- **やったこと (光源のドラッグ):** `viewport/tools/lighting/LightingOverlay.test.ts`。光源はモデルに入らないので、送るものはコマンドではなく
  `ipc.setPreviewLighting` (`LightingView.commit`) で、これを `vi.mock` で数える。平行光源 (円の中心 → 縁で向きが `[-1, 0, 0]`) と
  点光源 (キャンバス座標で動き、奥行きは保つ) のドラッグで、ドラッグ中は表示 (`view.settings` と取っ手の位置) だけが変わって送らず、
  離したときに 1 回だけ送ること、`pointercancel` で送らずに押したときの設定に戻すこと、動かさない押下と主でないボタンでは送らないこと。
- **やったこと (読めないモデル):** `doc.model.json` を読めない文字列にして描く。最初から読めない場合と、読めるモデルで描いた後で読めなくなる
  (読み直しの JSON が壊れている) 場合の両方。バックエンドは答えない `ipc` の偽物 (`Proxy`) にした。
  - 重ねもの (`viewport/unreadableModel.test.ts`): `ToolOverlay` を深度・はみ出し・物理・グルー (組の追加モード) をすべて出す状態で、
    選択なし・メッシュ (`FormTool`)・メッシュ編集 (`MeshEditTool`)・ワープと回転のデフォーマ (`DeformerGizmo`) の 5 通り。
    これで `FormTool`・`MeshEditTool`・`DeformerGizmo`・`DepthOverlay`・`OverflowOverlay`・`PhysicsOverlay`・`GlueOverlay` と
    `ViewportScene` を通る。ほかに `LightingOverlay` を単独で。モデルから作る円 (頂点・取っ手) が出ないこと・何も送らないことも確かめる
    (衝突体は `doc.physics` から描くので数えない)。`DepthOverlay` の null の確かめを外すと 8 件が落ちることを確かめた。
  - パネル (`panels/unreadableModel.test.ts`): `TreePanel`・`InspectorPanel` (メッシュ Quad を選んだ状態)・`ParametersPanel`・`PhysicsPanel`・
    `LightingPanel`・`GluesPanel`・`SlotsPanel`・`PosesPanel`・`VerifyPanel`・`TimelinePanel`。
  - 対象にしていないもの: `PreviewPanel`・`DifferenceOverlay` (プレビュー。wasm と `PreviewMode` が要る)、`App.svelte`、
    `Viewport.svelte` (WebPuppet の読み込み)、AI タブの部品。
- 部品のテストの道具 (`harness.svelte.ts`) に、パネルが読む状態を足した: `FakeDocument` の `expressions`・`warnings`・`dismissWarnings`、
  `FakeController` の `posePreview`・`setPosePreview`。テストのモデルのデフォーマに名前 (`Warp`・`Neck`) を付けた (物理のテンプレートの節が
  `name` を読む。本物のモデルには必ずある)。
- `crates:` の外の変更: `docs/testing.md#エディタの部品のテスト` に「読めないモデル」のテストの置き場を 1 行書いた。
- 部品の不具合は見つからなかった (T0698 で null の扱いがそろっていたため、直したコードは無い)。
- `just ci` が通った (Biome の警告 1 件 `bindings/unity/release.mjs` の未使用の import は main からのもので、この変更とは関係ない)。
- **起票の候補:** プレビュー (`PreviewPanel`・`DifferenceOverlay`) を読めないモデルで描くテスト (wasm と `PreviewMode` の偽物が要る)。
