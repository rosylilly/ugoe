---
id: T1648
title: "エディタ: メッシュの頂点を足す操作とグルーの 1 組ずつの編集に、ポインタ以外の代わりの手段を作る (判断が要る)"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: [T1647, T1642]
assignee: claude-worker-T1648
---

# T1648: エディタ: メッシュの頂点を足す操作とグルーの 1 組ずつの編集に、ポインタ以外の代わりの手段を作る (判断が要る)

## 目的

docs/editor.md の「ビューポートの操作と代わりの手段」の表で、次の 2 つは代わりが無い (T0675 の残り。T1635 の候補)。

- メッシュの編集: Ctrl + クリックで頂点を足す (位置を指す操作。今はインスペクタのメッシュの作り直しで形を作り直すだけ)
- グルー: 頂点を押して組を足す・外す (グルーのパネルの「距離で組み直す」だけで、1 組ずつは無い)

回転のギズモの回転・拡大率の取っ手は T1636 の数値の欄が代わりになった。

**判断が要る:** 頂点を足す位置の決め方と、グルーの組の選び方に案がある。

- 頂点: A. 選んだ 2 つの頂点の中点 (辺を分ける) に足す。B. ビューの中央に置いたカーソル (矢印で動かす) の位置に足す。C. インスペクタに座標の欄を置いて足す。
- グルーの組: D. グルーのパネルに組の一覧を置き、組ごとに外す・2 つのメッシュの頂点の番号を選んで足す。E. ビューポートで `]` `[` (T1635) で頂点を選び、キーで組を足す・外す。

推奨は、ほかのツールの例 (Blender の辺の分割など) と、既存のキーの割り当て (docs/editor.md のショートカット) と重ならないかを見てから書く。

## 受け入れ条件

- [x] 案と推奨をメモに書き、利用者の判断を得た
- [x] 決めた形で、マウスを使わずに頂点を足せ、グルーの組を 1 つずつ足し外しできる。どちらもコマンド 1 つ (Undo が 1 回で戻る)
- [x] 部品のテスト (キーかパネルの操作で送るコマンドとその値)
- [x] docs/editor.md の代わりの手段の表の「無し」「1 組ずつは無い」を置き換え、マニュアルの更新の候補をメモに書いた
- [x] `just ui-audit --only editor` の指摘が増えない。`just e2e --only editor` と `just ci` が通る

## 参考

- T1635 のメモ (起票の候補)、T0675 のメモ (代わりの手段の一覧)、T1636
- docs/editor.md#ビューポートの操作と代わりの手段、apps/editor/src/lib/tools (MeshEditTool)、apps/editor/src/lib/panels/glues/

## メモ

- T0043 で起票した (元の候補: T1635 (メッシュ編集の頂点を足す操作・グルーの 1 組ずつのキーボードでの代わり))。T1635 の候補のうち
  回転のギズモの取っ手は T1636 の数値の欄で代わりがあるので含めない。ビューポートの点の選択 (T1647) とグルーのパネル (T1642) の後にした。
- 判断 (2026-10-06): 利用者が頂点は案 A (`]` `[` で選んだ 2 つの頂点の中点に足す。辺を分ける)、グルーの組は案 D (グルーのパネルに組の一覧を置き、組ごとに外す・2 つのメッシュの頂点の番号を選んで足す) を選んだ。
- やったこと (2026-10-06):
  - **頂点 (案 A):** メッシュ編集で Shift + `]` `[` を「1 つ目 (選択の先頭) を残し、2 つ目を次・前の頂点へ」にした (`keys.ts` の
    `pointExtendOf`・`extendedPoints`。1 つ目は飛ばす。Shift + `]` の `key` は配列によって `}` なので `}` `{` `]` `[` を受ける)。2 つを選ぶと
    帯の読み上げの領域に「Points 1 and 2 of 4」(`viewport.point.pair`)。`I` (修飾キー無し。`isSplitKey`) か道具の並びの「Add midpoint」
    (2 つを選んだときだけ押せる。Tooltip にショートカット I) で、2 つの表示の位置の中点に `add_mesh_vertex` を 1 つ送り、足した頂点を選ぶ
    (`MeshEditTool.addMidpoint`)。ポインタで選んだ 2 つでも同じ。
  - **キーの割り当て:** `I` (Insert)。docs/editor.md のショートカット (Ctrl + Z・Y・S・O・Shift + S、Ctrl + A、Delete・Backspace、Esc、
    矢印・Alt + 矢印、`]` `[`) とアプリの中の 1 文字のキー (無し) に重ならない。Blender の辺の分割 (Subdivide) は既定のキーが無く
    メニューから選ぶので、合わせる例は無かった。Shift + `]` `[` は T1635 の `]` `[` の延長として覚えやすい。
  - **グルーの組 (案 D):** グルーのパネルの編集の欄に「Pairs」を置いた。組を `A 2 ↔ B 1` (頂点の番号は「Point 3 of 12」と同じく 1 から) で
    並べ、組ごとの「×」(読み上げは「Remove the pair of vertex 2 of Quad and vertex 1 of Quad2」) で外す (`withoutPair`)。「Vertex of A」
    「Vertex of B」の欄に番号を入れて「Add pair」で足す (`withPair`。B の頂点の前の組は置き換わる)。空・整数でない・もうある組の間は押せない
    (`glues.ts` の `pairVertex`・`hasPair`。もうある組なら「These vertices are already a pair.」)。どちらも `set_glue` を 1 つ (Undo 1 回)。
    一覧は高さ 12rem でその中だけをスクロールする。本体・複製のグルーでは無効。
  - **Command は足していない:** 頂点は既存の `add_mesh_vertex` (位置を渡す)、組は既存の `set_glue` (グルーを丸ごと置き換える) で
    どちらも 1 つで足りたので、ugoe-editor-core は変えていない (crates の外の変更は無し)。
  - 文言は英語と日本語の両方 (`glues.pairs.*`・`viewport.meshEdit.addMidpoint.*`・`viewport.point.pair`・`viewport.meshEdit.hint`)。
  - 部品のテスト: `ToolOverlay.test.ts` (Shift + `]` `[` の選択と読み上げ、`I` とボタンで送る `add_mesh_vertex` の位置、1 つのときは送らない)、
    `GluesPanel.test.ts` (一覧の表示・「×」で送る `set_glue` の組・番号から足す `set_glue` と欄を空にする・もうある組と整数でない番号)、
    `keys.test.ts`・`glues.test.ts` (純粋な関数)。
  - docs/editor.md: 代わりの手段の表の 2 行、`]` `[` の節の後に「2 つの頂点の中点に足す」、メッシュ編集の節、グルーの画面の「組の一覧」。
  - 確かめたこと: `just e2e --only editor` (15 件通過)、`just ui-audit --only editor` (rebase 前に 412 場面ですべての検査が 0。rebase で T0044 の `-glue-edit` の場面が入ったので、rebase 後は機械が混んでいて全体が終わらず、`--variants en-light-wide,ja-light-zoom200,ja-light-forced-narrow` の 120 場面 (`-glue-edit`・`-tab-glue`・`-mesh-edit` を含む) で流し、すべて 0)、`just ci` が通る。
- やらなかったこと: 内側の辺の中点に足したとき、その辺を残すとは限らない (`add_mesh_vertex` は足した点を含めて三角形を作り直す。
  輪郭の辺なら輪郭がその点で分かれる)。docs/editor.md にそう書いた。実機 (Tauri の窓) では動かしていない (部品のテストと e2e・ui-audit の偽のホストまで)。
- マニュアルの更新の候補 (起票の候補):
  - docs/manual/tools.md の「メッシュ編集」のキーの表に、Shift + ] / [ (2 つ目の頂点) と I・「Add midpoint」(2 つの中点に足す) を足す。
    122 行目の「キーボードだけで点を動かすとき」に、頂点を足す手順も書く。
  - docs/manual/dress-up.md のグルーの節 (231 行目あたり) に、パネルの「Pairs」の一覧 (「×」で外す・番号で「Add pair」) を足す。番号は
    メッシュ編集の `]` `[` の「Point 3 of 12」で確かめられることも。画像 (module-glue.png) の撮り直し。
- 起票の候補 (ほか): ビューポートの Glue の重ねで、パネルの組の行にフォーカスしたときにその組を強調する (番号と絵を結び付けやすく)。

