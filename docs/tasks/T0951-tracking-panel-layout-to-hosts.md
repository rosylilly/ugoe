---
id: T0951
title: "@ugoe/face-tracking: TrackingPanel から高さの上限と送りの指定を外し、置く側のアプリに任せる"
status: in-progress
milestone: M10
crates: [packages/face-tracking, apps/viewer, apps/studio, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0951
---

# T0951: @ugoe/face-tracking: TrackingPanel から高さの上限と送りの指定を外し、置く側のアプリに任せる

## 目的

`@ugoe/face-tracking` の `TrackingPanel` は自身に `max-height: 45%`・`overflow-y: auto` を持つ。T0725 でビューアは右の列を 2 段にし、パネルの高さの上限を
外から外した (`.features > *`)。配置は置く側 (ビューア・スタジオ・エディタ) が決めるものなので、部品から外す (T0725 の候補)。

## 受け入れ条件

- [ ] `TrackingPanel` が高さの上限と送りを持たない。ビューアの `.features > *` の打ち消しを外した
- [ ] スタジオとエディタでトラッキングのパネルを置く場所が、必要なら自分で高さと送りを決め、`just ui-audit` (3 つのアプリ) の指摘が増えない。前と後の画像を比べてメモに書いた
- [ ] packages/face-tracking/README.md (置き方) と docs/ui.md を直した
- [ ] `just ci` が通る

## 参考

- T0725 のメモ (やらなかったこと・起票の候補)、docs/ui.md#ビューアの見直し-t0725
- packages/face-tracking の `TrackingPanel`

## メモ

- T0040 で起票した (元の候補: T0725 (`TrackingPanel`))。パネルの中の詰め (「体 (肩)」の折り返し・小さなボタンの 2 行) も、3 つのアプリで画像を見て直せるならこのタスクで直す。
