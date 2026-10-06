---
id: T0948
title: "ビューアとスタジオのポーズのパネルを共有の部品にまとめる"
status: todo
milestone: M9
crates: [packages/player, apps/viewer, apps/studio, docs]
depends_on: [T0736]
assignee:
---

# T0948: ビューアとスタジオのポーズのパネルを共有の部品にまとめる

## 目的

ビューア (`apps/viewer/src/lib/PosePanel.svelte`、T0735) とスタジオ (`apps/studio/src/PosePanel.svelte`、T0842) のポーズのパネルは、
グループごとに `SegmentedControl` (5 個以上は `Select`) を出す作りが同じ (T0842 の候補)。表示する要素の出どころ (ビューアは `Player`、スタジオはバックエンドの通知) だけが違う。
1 つの部品にまとめ、T0736 (ビューアとエディタのプレビューの欄をモーションに追従させる) の振る舞いも揃える。

## 受け入れ条件

- [ ] グループと選んでいる要素と選んだときの処理を受け取る共有の部品が、`@ugoe/player` (か `@ugoe/ui`。置き場と理由をメモに書く) にあり、部品のテストがある
- [ ] ビューアとスタジオ (とエディタのプレビューにポーズの欄があればそれも) がその部品を使い、各アプリの `PosePanel.svelte` のテストが通る (重複したテストは部品の側へ移す)
- [ ] `just ui-audit --only viewer,studio` の指摘が増えない
- [ ] docs/ui.md (部品の一覧) と packages の README を直した
- [ ] `just ci` が通る

## 参考

- T0842 のメモ (起票の候補)、T0736
- apps/viewer/src/lib/PosePanel.svelte、apps/studio/src/PosePanel.svelte

## メモ

- T0040 で起票した (元の候補: T0842 (`PosePanel.svelte`))。ビューアのパネルを触る T0736 (作業中) の後に着手する。
