---
id: T0064
title: "ui-audit: scrollbar の検査で、帯の端のキーの印 (button.marker・button.diamond) を意図したものとして外す"
status: todo
milestone: M10
crates: [tests, apps/editor, docs]
depends_on: [T0979]
assignee:
---

# T0064: ui-audit: scrollbar の検査で、帯の端のキーの印 (button.marker・button.diamond) を意図したものとして外す

## 目的

T0055 の `scrollbar` の検査は、エディタの帯の端の値・時刻にあるキーの印 (パラメータのパネルの最大の値のキー `button.marker`、
ドープシートの終わりのキー `button.diamond`) を毎回数える。押せる範囲 (24px) を見た目の印 (◆) より広げて中心に置くので範囲の端が帯の端の先に
出るが、見た目の印は空きの中にあり、意図したもの (tests/ui-audit/README.md の「残る指摘 (意図したもの)」)。毎回出る指摘は新しい指摘を
埋もれさせるので、印を付けて検査から外すかを決める。

## 受け入れ条件

- [ ] 外し方を決め、理由をメモと tests/ui-audit/README.md に書いた。案: A (重なりの検査の `data-ui-audit-overlay` を scrollbar の検査でも見る)・
      B (scrollbar の検査だけの別の印を作る)・C (外さず、意図したものとして残す)。A は重なりの検査からも外れるので、印が重なりの検査に
      出ない (`position: absolute` など) ことを確かめてから選ぶ
- [ ] 外すなら、検査の判定 (`tests/ui-audit/lib/page-checks.js`) と `test/checks.test.mjs` のテストを足し、印を付けた所
      (ParametersPanel.svelte・MotionEditor.svelte) にコメントで理由を書いた
- [ ] `just ui-audit --only editor` (少なくとも `zoom200`・`narrow` の組) で `button.marker`・`button.diamond` の指摘が消え、
      ほかの指摘が変わらないことを `--compare` で確かめた
- [ ] `just ui-audit-check` と `just ci` が通る。クラス名や並びを変えたら `just e2e --only editor` も流した

## 参考

- T0055 のメモ (意図したもの)、T1669 のメモ (ドープシートの印に `data-ui-audit-overlay` を付けなかった理由)、T1668 (CurveGraph の `data-ui-audit-overlay`)
- tests/ui-audit/README.md の「重ねて描くスクロールバー」「押せる部品の重なり」、docs/ui.md (`data-ui-audit-overlay`)

## メモ

- T0063 で起票した (元の候補: T0055 (帯の端のキーの印が scrollbar の検査に毎回出る))。
- 同じ tests/ui-audit/README.md の「重ねて描くスクロールバー」を直す T0979 の後にした。
