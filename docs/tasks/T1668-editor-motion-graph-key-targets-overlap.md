---
id: T1668
title: "エディタ: モーションのグラフで、近いキーの押せる範囲が重なる (200% の拡大で 0 秒と 1 秒のキーが約 2px 重なる)"
status: in-progress
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1668
---

# T1668: エディタ: モーションのグラフで、近いキーの押せる範囲が重なる (200% の拡大で 0 秒と 1 秒のキーが約 2px 重なる)

## 目的

T1662 でエディタの送る枠の行末を `--ugoe-scrollbar-gap` で空けた結果、`en-light-zoom200` でモーションの編集の時刻の軸が 16px 狭くなり、
グラフのキー「Key at 0」と「Key at 1」の押せる範囲 (24px) が約 2px 重なる (ui-audit の `target-size` / 重なりの指摘が 1 件増えた)。
キーの間が 1 秒より近ければ、T1662 の前から重なる種類の問題 (T1662 の候補)。

## 受け入れ条件

- [ ] 近いキーの押せる範囲の扱いを決め、理由をメモと docs/ui.md (押せる大きさ) に書いた。案: 行の名前の列の最小 (150px) を縮める・重なるときは押せる範囲を中点で分ける (WCAG 2.5.8 の「間隔」の例外の考え方)・キーを選ぶ代わりの手段 (キーボード) があることを根拠に例外にする
- [ ] 決めた形で直し、部品のテストか ui-audit で確かめた
- [ ] `just ui-audit --only editor --variants <zoom200 を含む組>` で指摘が増えない (T1662 で増えた 1 件が消える)。`just e2e --only editor` と `just ci` が通る

## 参考

- T1662 のメモ、apps/editor のモーションの編集 (MotionEditor・CurveGraph)

## メモ

- T1662 のワーカーの報告から起票した (2026-10-07)。
