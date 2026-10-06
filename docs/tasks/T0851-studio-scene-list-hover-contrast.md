---
id: T0851
title: "スタジオ: ダークでシーンの一覧の選んだ行にホバーすると、控えめな文字が 4.5:1 に届かないのを直す"
status: done
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

- [x] 直し方を決めてメモに書いた (選んだ行の中の控えめな文字を地に合わせた色にする、か、`accent-soft` の上のホバーの重ねを弱める。後者は `@ugoe/ui` のトークン (T0916) なので、ほかの部品の選んだ行も見る)
- [x] `just ui-audit --only studio` のコントラストの指摘 (この行) が無くなり、ほかの指摘が増えない (`--compare` の結果をメモに書いた)。トークンを変えたならエディタ・ビューアの監査も増えない
- [x] docs/ui.md の「残したもの」からこの項を外した
- [x] `just ci` が通る

## 参考

- T0952・T0918・T0849 のメモ (起票の候補)
- apps/studio/src/ScenesPanel.svelte、packages/ui (トークン `--ugoe-overlay-hover`・`--ugoe-color-accent-soft`)、docs/ui.md

## メモ

- T0043 で起票した (元の候補: T0952 (シーンの一覧の選んだ行の hover)・T0918 (同じ。4.29)・T0849 (同じ))。
- 直し方: 選んだ行の中の控えめな文字 (モデルの名前・`Kbd` の「+」・「読み込み中」) を、text-muted に本文の色を 30% 混ぜた色にした
  (`ScenesPanel.svelte` の行の変数 `--scenes-muted`。`Kbd` の「+」は `.scenes__key :global(.ugoe-kbd__plus) { color: inherit }`)。
  ダークで地だけ 6.33・hover 5.39・押下 4.56 (前は 5.03・4.29・3.63)、ライトで 7.24・6.44・5.70。トークン (`--ugoe-overlay-hover`) は変えていない。
  理由は docs/ui.md の「スタジオのシーンの一覧の選んだ行 (T0851)」(重ねは地によらず同じ段の決め。@ugoe/ui の部品で accent-soft に重ねと text-muted が乗る所は無い)。
- `just ui-audit --only studio --compare target/ui-audit-before` (前は rebase 前の main cf09669d): `studio: +0 -2 ~0`
  (`span.scenes__model`・`span.ugoe-kbd__plus` の 4.29 が消え、コントラストの指摘は 0 件)。トークンを変えていないのでエディタ・ビューアの監査は流していない。
- `just e2e --only studio`: 13 件すべて通過。`just ci` 通過。
- 文字の色は CSS だけの変更なので単体のテストは足していない (監査が確かめる)。
- 起票の候補: エディタのツリー (`TreeItem.svelte`) で、非表示 (`.row.hidden`)・本体 (`.row.base`) の要素を選んだ行に hover すると、
  名前 (text-muted) が accent-soft + hover の重ねの上でダーク 4.29 (計算)。監査にその場面が無い。同じ直し方 (選んだ行では本文の色へ寄せる) が使える。
