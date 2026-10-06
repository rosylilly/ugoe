---
id: T0851
title: "スタジオ: ダークでシーンの一覧の選んだ行にホバーすると、控えめな文字が 4.5:1 に届かないのを直す"
status: in-progress
milestone: M10
crates: [apps/studio, packages/ui, docs]
depends_on: []
assignee: claude-worker-T0851
---

# T0851: スタジオ: ダークでシーンの一覧の選んだ行にホバーすると、控えめな文字が 4.5:1 に届かないのを直す

## 目的

スタジオのダークで、シーンの一覧 (`ScenesPanel.svelte`) の選んだ行 (地は `accent-soft`) にホバーの重ね (`--ugoe-overlay-hover`) が乗ると、
モデルの名前 (`.scenes__model`、text-muted) と `Kbd` の「+」が 4.29 で 4.5 に届かない (`just ui-audit` の `ja-dark-narrow-approved`。
承認のボタンを押した後のポインタが行の上に来る)。T0952・T0918・T0849 で見つかり、docs/ui.md の「残したもの」にある (バグ寄り)。

## 受け入れ条件

- [ ] 直し方を決めてメモに書いた (選んだ行の中の控えめな文字を地に合わせた色にする、か、`accent-soft` の上のホバーの重ねを弱める。後者は `@ugoe/ui` のトークン (T0916) なので、ほかの部品の選んだ行も見る)
- [ ] `just ui-audit --only studio` のコントラストの指摘 (この行) が無くなり、ほかの指摘が増えない (`--compare` の結果をメモに書いた)。トークンを変えたならエディタ・ビューアの監査も増えない
- [ ] docs/ui.md の「残したもの」からこの項を外した
- [ ] `just ci` が通る

## 参考

- T0952・T0918・T0849 のメモ (起票の候補)
- apps/studio/src/ScenesPanel.svelte、packages/ui (トークン `--ugoe-overlay-hover`・`--ugoe-color-accent-soft`)、docs/ui.md

## メモ

- T0043 で起票した (元の候補: T0952 (シーンの一覧の選んだ行の hover)・T0918 (同じ。4.29)・T0849 (同じ))。
