---
id: T1653
title: "エディタ: ] [ で選んだ点を見せるパンの判定に、上の帯と右下の道具を入れる (見える矩形で判定する)"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1653
---

# T1653: エディタ: ] [ で選んだ点を見せるパンの判定に、上の帯と右下の道具を入れる (見える矩形で判定する)

## 目的

T1647 で `]` `[` で選んだ点がビューの外にあるとき見える所までパンするようにした (`revealPanOf`。端から 48px の余白)。
余白はビューポートの全体の矩形から測り、左上の道具の並びと説明の帯 (`.strip`)・右下の拡大の道具の高さを見ていない。帯が高いと、
上の端の点が帯の下に隠れたままになる (T1647)。

## 受け入れ条件

- [x] 判定とパンの量を、ビューポートで実際に見える矩形 (上の帯と右下の道具を除いた範囲。大きさが変わったら測り直す) で求める
- [x] 純粋な関数のテスト: 見える矩形の内側では動かさない・帯の下に入る点は帯の下の端から余白の所まで動かす・見える矩形が余白の 2 倍より狭い軸は真ん中へ
- [x] 部品のテスト: 帯の高さを持たせた ToolOverlay (か ViewportController) で、帯の下の点を `]` で選ぶと表示が動く
- [x] docs/editor.md の「選んだ点を見える所へ」を直した
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T1647 のメモ (やらなかったこと・起票の候補)
- apps/editor/src/lib/tools/keys.ts (`revealPanOf`)、apps/editor/src/lib/viewport (`ViewportController.reveal`)、docs/editor.md#ビューポートの操作と代わりの手段

## メモ

- T0046 で起票した (元の候補: T1647 (パンの判定に帯と道具の高さを入れる))。
- やったこと:
  - `tools/keys.ts`: `visibleRectOf(size, covered)` (ビューの大きさから上の端・下の端を隠す高さを除いた見える矩形。隠す高さが
    数でない・負なら 0、上下を合わせて高さ以上を隠すならビュー全体) と、`revealPanOf` を見える矩形 (`ViewRect`) で判定するように変えた。
  - `ViewportController.setCovered("top" | "bottom", height)` を足し、`reveal` は `visibleRectOf` の矩形で判定する。
    ToolOverlay が `.strip` の `offsetHeight` を (大きさが変わるたびに) 上として渡し、外すときは 0 に戻す。Viewport が右下の
    表示の道具の `offsetHeight` + 8px (CSS の `bottom`) を下として渡す (道具を出していない間は 0)。
  - テスト: keys.test.ts (見える矩形の内側では動かさない・帯の下の点は帯の下の端から余白の所へ・道具に重なる点は上へ・狭い軸は真ん中へ・
    `visibleRectOf` の数でない値)、controller.test.ts (`setCovered` で帯の下へ)、ToolOverlay.test.ts (`.strip` の高さ 200px で
    `]` で選んだ帯の下の頂点へパン)、Viewport.svelte.test.ts (表示の道具の高さを下に渡す・出していない間は 0)。偽の controller に
    `covered`・`setCovered` を足した。
  - 文書: docs/editor.md の「選んだ点を見える所へ」、docs/manual/tools.md。
  - `just e2e --only editor`: 15 件すべて通過。`just ci`: 通過。
- やらなかったこと:
  - 隠す範囲は上の端・下の端の帯として全幅で除く (右下の道具は右の角だけを隠すが、左下の点も道具の高さの分だけ上げる)。
    右上の HUD・左下の帯 (グルー・違いの知らせ)・提案のプレビューの帯 (App.svelte の `.proposal-banner`) は入れていない。
  - 実物の WebView での見た目は確かめていない (happy-dom は大きさを測らないので、部品のテストは `offsetHeight` を差し替えた)。
- 起票の候補: 提案のプレビューの帯 (`.proposal-banner`) が出ている間も、その高さを上の隠す範囲に入れる (App.svelte。今は .strip と重なる)。
