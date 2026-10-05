---
id: T1624
title: "エディタ: プレビュー (PreviewPanel・DifferenceOverlay) を読めないモデルで描く部品のテスト"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1624
---

# T1624: エディタ: プレビュー (PreviewPanel・DifferenceOverlay) を読めないモデルで描く部品のテスト

## 目的

T1613 で、ビューポートの重ねものとパネルを読めないモデル (`doc.model.json` が壊れている) で描く部品のテストを足した
(`viewport/unreadableModel.test.ts`・`panels/unreadableModel.test.ts`)。プレビュー (`apps/editor/src/lib/preview/PreviewPanel.svelte`・
`DifferenceOverlay.svelte`) は wasm と `PreviewMode` が要るので対象にしなかった (T1613 の候補)。

## 受け入れ条件

- [x] `PreviewPanel`・`DifferenceOverlay` を、最初から読めないモデルと、読めるモデルで描いた後で読めなくなるモデルで描くテストがある (wasm と `PreviewMode` は偽物)。例外を出さず、何も送らない
- [x] テストの道具 (`harness.svelte.ts`) に足したものを docs/testing.md#エディタの部品のテスト の「読めないモデル」の行に書いた
- [x] 見つかった不具合は直した (テストが先に落ちることを確かめた) — 不具合は見つからなかった (メモ)
- [x] `just ci` が通る

## 参考

- T1613 のメモ (起票の候補)
- docs/testing.md#エディタの部品のテスト

## メモ

- T0036 で起票した (元の候補: T1613)。
- **やったこと:** `apps/editor/src/lib/preview/unreadableModel.test.ts`。`doc.model.json` を読めない文字列にして、最初から読めない場合と、
  読めるモデルで描いた後で読めなくなり、また読めるようになる場合を描く。
  - `PreviewPanel`: 光源の節 (`viewport.usesLighting`)・ポーズのグループ・スロット・モーション・表情・パラメータを出す状態。読めない間は
    モデルから作るもの (ポーズのグループ) を出さず、ドキュメントのもの (パラメータ) は出す。
  - `DifferenceOverlay`: メッシュ・ワープ・回転・衝突体の違いを強調した状態 (WebPuppet の偽物はモジュール `Mod/` の接頭辞の ID を返す)。
    メッシュは、読めるうちは作業用のモデルの三角形の外周、読めない間は凸包で描くことも確かめる。
  - どれも、コマンド (`doc.apply`)・プレビューの操作 (`FakePreview.calls`)・バックエンド (`ipc` の `Proxy` で記録)・ビューの移動
    (`zoomAt`・`panBy`) を何も呼ばないことを確かめる。
  - `PreviewPanel` の `model?.pose` と `DifferenceOverlay` の `summarizeModel(doc.parsedModel)` を null を許さない形に変えると 9 件が
    落ちることを確かめた (変更は戻した)。衝突体の 2 件はメッシュを読まないので落ちない。
- テストの道具: `PreviewMode` の偽物 `FakePreview` を `apps/editor/src/lib/preview/test/fakePreview.svelte.ts` に置いた (トラッカーと
  `DressUp` は本物。`@ugoe/face-tracking` の読み込みが重いので `harness.svelte.ts` に入れない)。`harness.svelte.ts` には
  `FakeController` の `message`・`usesLighting`・`zoomAt`・`panBy`、`fakePuppet` の `has_model`・`attached_modules`・`physics_json` を足した。
  テストのモデル `testModel` は変えず、プレビューのテストの中で `meta`・`effects`・`pose`・`slots`・`avatar` を足したモデルを作る。
- `docs/testing.md#エディタの部品のテスト` の「読めないモデル」の行に、置き場と道具を書いた。
- 部品の不具合は見つからなかった (直したコードは無い)。
- `just ci` が通った。
- **起票の候補:** なし (`App.svelte`・`Viewport.svelte`・AI タブの部品は T1613 のメモのとおり対象外のまま)。
