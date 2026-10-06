---
id: T1664
title: "エディタ: ビューポートでグルーの組にポインタを乗せると、パネルの組の行を強調して一覧をその行へ送る"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: [T0975]
assignee: claude-worker-T1664
---

# T1664: エディタ: ビューポートでグルーの組にポインタを乗せると、パネルの組の行を強調して一覧をその行へ送る

## 目的

T1651 で、グルーのパネルの組の行にフォーカス・ポインタを置くと、ビューポートでその組を強調するようにした (パネル → ビューポート)。
逆向き (ビューポートの組の線・頂点 → パネルの行) が無いので、絵で見つけた組が一覧のどの行かが分からない。双方向に結び付ける。

## 受け入れ条件

- [x] ビューポートのグルーの重ねで組の線か頂点にポインタを乗せると、パネルのその組の行を強調し (T1651 の行の強調と同じ見た目)、
      一覧がその行を見える所まで送る (送るのはポインタが止まったとき・行が見えていないときだけなど、一覧が暴れない条件を決めてメモに書いた)。
      離れると戻る。ビューポートの側でもその組を強調する (T1651 の `highlightedPair` に合わせる)
- [x] 強調は表示だけで、Command を送らない。「Add pairs」「Remove pairs」の道具の操作 (押して組を足す・外す) を妨げない
- [x] 部品のテスト: 重ねの組に乗せると強調する組が決まり、パネルの行が強調されて送られる。離すと消える
- [x] docs/editor.md のグルーの画面 (「組の強調」) と docs/manual/dress-up.md のグルーの節に逆向きの強調を書いた
- [x] `just ui-audit --only editor` (`-glue-edit` を含む組) で増えた指摘が無い。`just e2e --only editor` と `just ci` が通る

## 参考

- T1651 のメモ (やったこと・起票の候補 (ほか))
- apps/editor/src/lib/panels/glues/ (GluesPanel.svelte・glueView.svelte.ts)、ビューポートのグルーの重ね (GlueOverlay)

## メモ

- T0052 で起票した (元の候補: T1651 (逆向き: ビューポートの組 → パネルの行))。
- docs/manual/dress-up.md のグルーの節を書き足す T0975 の後にした。
- やったこと (2026-10-07):
  - `GlueView` に `viewportPair` (ビューポートで指している組) を足し、`highlightedPair` をパネルの行のポインタ → ビューポートのポインタ →
    フォーカスの順にした。パネルの行のポインタ (`hoveredPair`) とは別に持つので、片方が離れてももう片方を消さない。`clearHighlight` も消す。
  - GlueOverlay は window の `pointermove` で受け、組の線か両端から 6 CSS ピクセル (`PAIR_HOVER_RADIUS`) 以内でいちばん近い組を
    `viewportPair` にする (`geometry.ts` に `segmentDistance` を足した)。重ねの svg は `pointer-events: none` のままで、イベントは止めない
    (メッシュの選択・「Add pairs」「Remove pairs」の押す操作を妨げない)。ボタンを押している間とタッチは決め直さない。ポインタが絵
    (ビューポートの中の svg かキャンバス) の上に無いとき (道具のボタン・帯・グルーの帯の上、ビューポートの外) と、窓の外へ出たとき・
    重ねを出さなくなったときは消す。強調の描き方は T1651 の `g.highlight` のまま。
  - 一覧を送る条件: ポインタが同じ組に 250 ms (`PAIR_SCROLL_DELAY`) 留まり、かつ行が一覧 (`.pair-list`) の見える所から外れているときだけ、
    近い側の端に合わせて一覧の中だけを送る (`scrollIntoView` はパネル全体も動かすので使わない)。通り過ぎる・行が見えているときは動かさない。
  - 部品のテスト: `GlueOverlay.test.ts` (線・頂点の近くで組が決まり強調する、離すと消える、押している間は決め直さない、帯のボタンの上では
    消す、「Remove pairs」の印を押す操作はそのまま、重ねが消えたら忘れる)、`GluesPanel.test.ts` (行の強調と消去・Command を送らない、
    留まったら隠れた行まで送る・見えている行では動かさない・上に隠れた行は上端に合わせる、留まる前に離れたら送らない)、`geometry.test.ts`。
  - docs/editor.md のグルーの画面に「逆向きの強調」、docs/manual/dress-up.md のグルーの節に逆向きの強調を書いた。
  - 確かめたこと: `just e2e --only editor` (16 件通過)、`just ui-audit --only editor --variants en-light-wide,ja-light-zoom200,ja-light-forced-narrow`
    (139 場面、すべての検査が 0)、`just ci` (通過)。
- やらなかったこと: 実機 (Tauri の窓) では動かしていない (部品のテストと e2e・ui-audit の偽のホストまで)。ui-audit にビューポートで組を
  指した場面は足していない (行の見た目は T1651 の強調と同じ)。
- 起票の候補: なし。
