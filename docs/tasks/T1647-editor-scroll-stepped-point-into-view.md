---
id: T1647
title: "エディタ: ] [ で選んだ点がビューの外にあるとき、見える所まで表示を動かす"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1647
---

# T1647: エディタ: ] [ で選んだ点がビューの外にあるとき、見える所まで表示を動かす

## 目的

T1635 で `]` `[` でフォーム編集・ワープ・メッシュ編集の点を 1 つずつ選べるようになったが、選んだ点がビューの外にあると見えない
(Alt + 矢印か「全体を表示」で探すしかない。T1635 の候補)。キーボードだけで点を追えるようにする。

## 受け入れ条件

- [x] `]` `[` で選んだ点がビューの外 (または端の余白の内側) にあるとき、点が見える所までパンする。拡大率は変えない。ビューの中なら動かさない
- [x] ポインタでの選択では動かさない
- [x] 判定とパンの量の純粋な関数のテストと、部品のテスト (ビューの外の点で表示が動く・中では動かない)
- [x] docs/editor.md の代わりの手段の表 (「点を選ぶ」の行) を直した
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T1635 のメモ (やらなかったこと・起票の候補)
- apps/editor/src/lib/viewport (`ViewportController.panBy`)、apps/editor/src/lib/tools (`keys.ts` の `steppedPoint`)、docs/editor.md#ビューポートの操作と代わりの手段

## メモ

- T0043 で起票した (元の候補: T1635 (`]` `[` で選んだ点を見える所へ))。
- やったこと:
  - `tools/keys.ts` に `revealPanOf` (点とビューの大きさから動かす量。端から `REVEAL_MARGIN` = 48px の余白より内側なら null、外なら
    端から 48px の所まで軸ごとに足りない分だけ。余白の 2 倍より狭い軸は真ん中へ。NaN・無限大・大きさ 0 は null)。
  - `ViewportController.reveal(x, y)`: `resize` で覚えた大きさで `revealPanOf` を求め、`panBy` を呼ぶ (拡大率は変えない)。
  - `]` `[` で点を選んだ所だけで呼ぶ: FormTool (`onKeyDown`)・DeformerGizmo (`step`)・MeshEditTool (`step` から新しい props の
    `reveal`。ToolOverlay が `controller.reveal` を渡す)。ポインタでの選択の道筋は触っていない。
  - テスト: keys.test.ts (`revealPanOf`)、controller.test.ts (大きさを知る前は動かさない・中では動かさない・外ではずらす)、
    FormTool・DeformerGizmo・ToolOverlay の部品のテスト (外・余白の中の点で `panBy`、中の点とクリックでは呼ばない)。
    偽の `FakeController` に `size` と `reveal` を足した (`size` が null なら動かさないので、ほかのテストは変わらない)。
  - 文書: docs/editor.md の代わりの手段の表 (頂点・点を選ぶ 2 行) と「選んだ点を見える所へ」の説明、docs/manual/tools.md。
  - `just e2e --only editor`: 15 件すべて通過。`just ci`: 通過。
- やらなかったこと: 実物の WebView での見た目 (パンの後に点が帯 (`.strip`) の下に隠れないか) は確かめていない。余白の 48px は上の帯の
  高さを見ていない (帯が高いときは上の端の点が帯に重なることがある)。
- 起票の候補: 上の帯・右下の道具の高さを余白に入れる (ビューポートの実際に見える矩形で判定する)。
