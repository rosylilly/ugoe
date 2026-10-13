---
id: T0986
title: "トラッキング中に、見せない状態の文が送る枠の外を基準にして、スタジオの操作ウィンドウの全体が縦に送れる"
status: in-progress
milestone: M10
crates: [packages/face-tracking, docs]
depends_on: []
assignee: claude
---

# T0986: トラッキング中の見せない状態の文が送る枠からはみ出す

## 目的

Windows のスタジオで、カメラのトラッキングを始めると操作ウィンドウの全体にスクロールバーが出て、ウィンドウの高さほど余計に送れる
(トラッキングを止めると消える。利用者の実機の確認、2026-10-13)。

`TrackingPanel` の状態の文 (`.status`) は、トラッキング中と再生中に `.ugoe-visually-hidden` (`position: absolute`) にして読み上げにだけ残す (T0958)。
包む `.state` とその祖先に `position` が無いので、文は右のパネル (送る枠) の外を位置の基準にし、枠で切り取られずに文書の全体を伸ばしていた。
[docs/ui.md](../ui.md#レイアウト) の「送り」の規則 (見せずに読ませる文字は包む要素を基準にする。T1657) に合わせて直す。

## 受け入れ条件

- [ ] `TrackingPanel` の `.state` を見せない状態の文の位置の基準にした
- [ ] 送る枠の中の `TrackingPanel` で、トラッキング中に文書が送れないことをテストか検査で確かめる
- [ ] `just ci` が通る

## 参考

- docs/ui.md#レイアウト の「送り」、T0958・T1657、packages/face-tracking/src/TrackingPanel.svelte

## メモ

- 実機の確認表 (docs/manual-check.md) の項目を回しているときに、利用者が見つけた。
