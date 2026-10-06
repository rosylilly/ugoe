---
id: T1662
title: "エディタ: 送る枠の行末を --ugoe-scrollbar-gap にし、tokens.test.ts の検査の対象にエディタを足す"
status: done
milestone: M10
crates: [apps/editor, packages/ui, docs]
depends_on: []
assignee: claude-worker-T1662
---

# T1662: エディタ: 送る枠の行末を --ugoe-scrollbar-gap にし、tokens.test.ts の検査の対象にエディタを足す

## 目的

WebKitGTK のスクロールバーは幅を取らずに中身の上に重ねて描き、指を乗せると 6〜14px に太る (T0970)。T0970 はビューアとスタジオの送る枠の
行末 (右) の padding を `--ugoe-scrollbar-gap` (16px) にしたが、エディタ (範囲の外) は直していない。エディタのパネルの送る枠 (`.pane`・`aside`) は
右 12px で、指を乗せたときだけ 2px 掛かる。ほかにも空けていない枠がある。

## 受け入れ条件

- [x] エディタの縦に送る枠の行末を `--ugoe-scrollbar-gap` にした。少なくとも T0970 の一覧: パネルの `.pane`・`aside`、PickList、GluesPanel の組の一覧、
      LightingPanel のメッシュの一覧、MotionEditor、ExpressionEditor の表、FileMenu の警告の一覧、Viewport の空の案内、LicenseDialog。
      空けない枠は、理由をスクロールバーの言葉で CSS のコメントに書いた
- [x] packages/ui の `tokens.test.ts` の検査 (送る枠の規則が `--ugoe-scrollbar-gap` を使うか、使わない理由を書く) の対象にエディタを足し、
      空けていない規則が残っていると落ちることを確かめた
- [ ] 狭くなった中身で文字が省略・折り返しにならないかを `just ui-audit --only editor` (600 秒に収まらなければ `--variants` で分けて、すべての組) と
      `--compare` で確かめ、増えた指摘が無い
- [x] docs/ui.md (レイアウトの「送り」) を直した。`just e2e --only editor` と `just ci` が通る
- 確かめていないこと (WebView2・WKWebView の重ねて描くスクロールバー) はメモに書く

## 参考

- T0970 のメモ (やったこと・やらなかったこと・起票の候補)、docs/ui.md (`--ugoe-scrollbar-gap`)
- apps/editor/src (App.svelte の `.pane`・app.css の `aside`・上の一覧の部品)、packages/ui/src/tokens.test.ts

## メモ

- T0052 で起票した (元の候補: T0970 (エディタの送る枠に `--ugoe-scrollbar-gap`))。
- 作業中の T1654 (TreeItem.svelte) とは別のファイル。ツリーのパネルの送る枠を触るなら T1654 の後に rebase して確かめる。
- やったこと:
  - packages/ui の `tokens.test.ts` の「送る枠の行末」の検査の対象に apps/editor/src を足した。直す前に流すと、11 の規則
    (App.svelte の `.pane`・app.css の `aside`・FileMenu の `.warning-list`・LicenseDialog の `pre`・GluesPanel の `.pair-list`・
    LightingPanel の `.meshes`・PickList の `.options`・ExpressionEditor の `.table`・MotionEditor の `.motion-editor` と `.sheet`・
    Viewport の `.empty`) を名前で出して落ちることを確かめた。
  - 行末の padding を `--ugoe-scrollbar-gap` にした (エディタの compact では 12px → 16px など)。例外は 3 つで、CSS のコメントに書いた:
    ExpressionEditor の `.table` は中の表の右の margin で空ける。MotionEditor の `.motion-editor` (200% の拡大でだけ全体を送る) は
    中の段 (`.bar`・`.sheet`) が空ける。Viewport の `.empty` は中央に置くので両側を同じだけ空ける。
  - 本物の WebKitGTK (`just e2e --manual --only editor`。画像は target/e2e/editor/manual に撮り、docs/manual/images は書き換えていない) で、
    docs/manual/images の前の画像と比べた: screen.png のモーションの表の 4 秒のキーとパラメータのパネルの右端、lighting.png の
    メッシュの一覧の「N」のボタンが、直す前はスクロールバーの下にあり、直した後は離れている。
  - `just ui-audit --only editor` を `--variants en-` と `--variants ja-` に分けて (266 + 266 場面)、直す前の CSS を作業ツリーに一時的に
    戻して撮った基準と `--compare` で比べた: ja は +0 -0 ~0。en は +1 -0 ~0 (下)。
  - `just e2e --only editor` (筋書き 16) と `just ci` が通った。`just ci` の 1 回目は、rebase の前に作った wasm の生成物
    (gitignore 済み) が古く `wasm-types` で落ちたので、`just wasm` で作り直して通した (変更とは無関係)。
- 満たしていない受け入れ条件 (ui-audit の増えた指摘):
  - en-light-zoom200 の timeline-graph などの場面で `[overlap] rect@.graph "Key at 0: 0" is covered by rect@.graph "Key at 1: 8"`。
    200% の拡大ではモーションの編集の全体が送るので、表 (`.sheet`) とグラフの時刻の軸が 16px 狭くなり (グラフの幅は表の目盛りの幅に
    そろう)、4 秒のモーションの 0 秒と 1 秒のキーの押せる範囲 (24px) の間が約 25px から約 22px になって 2px ほど重なった。
    キーが 1 秒より近ければ直す前から重なる種類の指摘で、全体を送る場面ではスクロールバーが右端の 14px に重なるため、空けるのを
    やめるより残す方がよいと判断した。直すなら行の名前の列の最小 (150px) を縮めるか、近いキーの押せる範囲の扱いを決める必要がある。
  - 200% の拡大の左の列で、ツリーの名前「HairFront」が「HairFr…」と省略されるようになった (列の右の padding が 4px 増えたため。
    ツリーの名前の省略は作りどおりで、ui-audit の指摘にはならない)。
- 確かめていないこと: WebView2 (Windows)・WKWebView (macOS) のスクロールバー (重ねて描く形の macOS も同じ padding で避けられるはずだが未確認)。
- 起票の候補:
  - モーションの編集のグラフで、キーの押せる範囲 (24px) が重なるほど狭いとき (200% の拡大・短い間隔のキー) の扱い (上の指摘)。
