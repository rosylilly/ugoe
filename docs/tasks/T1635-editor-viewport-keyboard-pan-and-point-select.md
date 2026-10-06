---
id: T1635
title: "エディタ: ビューポートの移動と、フォーム編集・ワープの点の選択をキーボードでもできるようにする"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: [T0675]
assignee: claude-worker-T1635
---

# T1635: エディタ: ビューポートの移動と、フォーム編集・ワープの点の選択をキーボードでもできるようにする

## 目的

T0675 のビューポートの見直しで、ポインタだけでできる操作のキーボードでの代わりを表にしたが、作っていないものが残った (T0675 の候補):

- ビューポートの移動 (パン)。
- フォーム編集・ワープの点を 1 つずつ選ぶこと。

## 受け入れ条件

- [x] ビューポートにフォーカスがあるとき、キーボードで表示を動かせる (矢印など。キーと 1 回の量をメモに書く。入力欄の中では効かない)
- [x] フォーム編集・ワープの道具で、キーボードで次・前の点を選べ (Tab か `[` `]` など。選んだ点をビューポートと読み上げで分かるように)、選んだ点を矢印で動かせる
      (既にあるなら確かめてメモに書く)。点の移動は今のコマンドを通す
- [x] 単体テスト (キーと動き・選ぶ点の順) と部品のテストがある
- [x] docs/editor.md・docs/ui.md (キーボードでの代わりの表)・docs/manual (ショートカット) を直し、画面が変わったところを T0919 のメモに書き足した
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0675 のメモ (やらなかったこと・起票の候補)、docs/ui.md (キーボードでの代わりの表)
- apps/editor/src/lib/viewport

## メモ

- T0041 で起票した (元の候補: T0675 (ビューポートの移動)・T0675 (点を 1 つずつ選ぶ))。
- やったこと:
  - **パン:** Alt + 矢印キーで表示を動かす (1 回 40 CSS px、Shift も押して 200 px。Alt + ドラッグと同じく絵が矢印の向きに動く)。
    `Viewport.svelte` が window の keydown で受け、`ViewportController.panBy` を呼ぶ。判定は `tools/keys.ts` の `panOf`。
    効くのは `acceptsNudgeKey` と同じ所 (フォーカスがどこにも無いかビューポートの中) だけで、入力欄・ダイアログ・ツリー・スライダー・
    タイムライン・パラメータのグループの Alt + ↑ / ↓ (T1630) とは重ならない。Alt の無い矢印 (点の移動) とも重ならない。
  - **点を 1 つずつ選ぶ:** `]` で次、`[` で前の点を 1 つだけ選ぶ (`pointStepOf`・`steppedPoint`。番号の順で端は反対の端へ。選んだ点が無ければ
    `]` は最初・`[` は最後から、あれば選択の末尾から)。Tab はフォーカスの移動に残した。フォーム編集 (FormTool)・ワープのギズモ
    (DeformerGizmo)・メッシュ編集 (ToolOverlay → `MeshEditTool.step`) で効く (メッシュ編集は範囲の外だが同じ表の行なので足した)。
    1 つだけ選んだ点は輪 (`.ring`) で囲み、帯の `role="status"` に「Point 3 of 12」(`viewport.point.current`) を出して読み上げる。
  - **選んだ点を矢印で動かす:** 既にあった (T0675。フォーム編集・ワープは `set_keyform_cell`、メッシュ編集は `move_mesh_vertices`)。
    `]` `[` で選んだ点にもそのまま効くことを FormTool のテストで確かめた。
  - テスト: keys.test.ts (キーと動き・選ぶ点の順)、Viewport.svelte.test.ts (パンとフォーカスの場所)、FormTool・DeformerGizmo・ToolOverlay の
    部品のテスト (選ぶ順・読み上げの文・輪・ビューポートの外では奪わない・回転デフォーマでは使わない)。
  - 文書: docs/editor.md の代わりの手段の表と説明、docs/ui.md (キーボードの基準に表への参照と、T0675 の節に T1635 の追記)、
    docs/manual/tools.md・screen.md、T0919 のメモ。メッシュ編集の帯の説明に「[ ]: pick next / previous」を足した。
  - `just ui-audit --only editor --compare` (前は f7e1905f の apps/editor で撮った): +0 -0 ~0。`just e2e --only editor`: 15 件すべて通過。`just ci`: 通過。
- やらなかったこと: `]` `[` で選んだ点がビューの外にあるとき、見える所まで表示を送ること (Alt + 矢印か「全体を表示」で見る)。
  実物の WebView (WebKitGTK・WebView2・WKWebView) とスクリーンリーダーでの読み上げは確かめていない (happy-dom の部品のテストとヘッドレスの Chrome の監査だけ)。
  Windows の WebView2 で Alt を押して離したときにメニューに飛ばないかも確かめていない (keydown で preventDefault している)。
- 起票の候補:
  - `]` `[` で選んだ点がビューの外なら、見える所まで表示を動かす。
  - 回転のギズモの回転・拡大率の取っ手、メッシュ編集の頂点を足す操作、グルーの 1 組ずつのキーボードでの代わり (T0675 の残り)。
