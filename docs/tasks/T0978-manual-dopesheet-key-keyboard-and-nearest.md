---
id: T0978
title: "マニュアル: ドープシートのキーの印を Enter・Space でも選べることと、近いキーは押した所に近い方を選ぶことを書き足す"
status: done
milestone: M10
crates: [docs]
depends_on: []
assignee: claude-worker-T0978
---

# T0978: マニュアル: ドープシートのキーの印を Enter・Space でも選べることと、近いキーは押した所に近い方を選ぶことを書き足す

## 目的

T1669 で、ドープシートのキーの印は Enter・Space でも選べ (再生位置もその時刻へ)、押せる範囲が重なる所では押した所に近いキーをつかむようになった。
T1668 でモーションのグラフの印も近い方をつかむ。マニュアルにはまだ書いていない (T1669 の候補)。

## 受け入れ条件

- [x] docs/manual/motions.md の「行のキーの印を押す」と docs/manual/tools.md の「キーの印のドラッグ」(とグラフの印の節) に、上の 2 つを書いた。文言とキーはコードと docs/editor.md で確かめた
- [x] `just ci` が通る

## 参考

- T1668・T1669、docs/editor.md

## メモ

- T1669 のワーカーの報告から起票した (2026-10-07)。
- やったこと:
  - docs/manual/motions.md: 「行のキーの印を押す」に Enter・Space でも選べることと、印が重なる所では押した所に近いキーを選ぶことを書き足した。
    表の下に、印にフォーカスがあるときの Space は再生・停止ではなくキーを選ぶこと、時刻を変えるドラッグはポインタだけで
    キーボードではグラフの左の「t」に入れることを足した。「カーブと取っ手」の表の下に、円や四角が重なる所では押した所に中心が近い方をつかむことを足した。
  - docs/manual/tools.md の「タイムライン」: 「キーの印を押す (ドープシートの印は、フォーカスして Enter・Space でも)」の行と、
    印 (ドープシートの印・グラフの円と四角) が重なる所では近い印をつかむ注記を足した。
  - 文言は `MotionEditor.svelte` (`pickKey`・印の `onkeydown` の Enter・Space、`nearestMark`)、`CurveGraph.svelte` (`onDown` の `nearestMark`)、
    docs/editor.md のドープシートの行、docs/ui.md の「近い印の重なり」で確かめた。グラフの印はフォーカスが止まらない (`tabindex="-1"`) ので、
    Enter・Space はドープシートの印だけに書いた。`just ci` が通った。
- やらなかったこと: 画像は撮り直していない (見た目は変わらない)。
- 起票の候補: なし。
