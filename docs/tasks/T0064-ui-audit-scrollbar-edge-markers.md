---
id: T0064
title: "ui-audit: scrollbar の検査で、帯の端のキーの印 (button.marker・button.diamond) を意図したものとして外す"
status: done
milestone: M10
crates: [tests, apps/editor, docs]
depends_on: [T0979]
assignee: claude-worker-T0064
---

# T0064: ui-audit: scrollbar の検査で、帯の端のキーの印 (button.marker・button.diamond) を意図したものとして外す

## 目的

T0055 の `scrollbar` の検査は、エディタの帯の端の値・時刻にあるキーの印 (パラメータのパネルの最大の値のキー `button.marker`、
ドープシートの終わりのキー `button.diamond`) を毎回数える。押せる範囲 (24px) を見た目の印 (◆) より広げて中心に置くので範囲の端が帯の端の先に
出るが、見た目の印は空きの中にあり、意図したもの (tests/ui-audit/README.md の「残る指摘 (意図したもの)」)。毎回出る指摘は新しい指摘を
埋もれさせるので、印を付けて検査から外すかを決める。

## 受け入れ条件

- [x] 外し方を決め、理由をメモと tests/ui-audit/README.md に書いた。案: A (重なりの検査の `data-ui-audit-overlay` を scrollbar の検査でも見る)・
      B (scrollbar の検査だけの別の印を作る)・C (外さず、意図したものとして残す)。A は重なりの検査からも外れるので、印が重なりの検査に
      出ない (`position: absolute` など) ことを確かめてから選ぶ
- [x] 外すなら、検査の判定 (`tests/ui-audit/lib/page-checks.js`) と `test/checks.test.mjs` のテストを足し、印を付けた所
      (ParametersPanel.svelte・MotionEditor.svelte) にコメントで理由を書いた
- [x] `just ui-audit --only editor` (少なくとも `zoom200`・`narrow` の組) で `button.marker`・`button.diamond` の指摘が消え、
      ほかの指摘が変わらないことを `--compare` で確かめた
- [x] `just ui-audit-check` と `just ci` が通る。クラス名や並びを変えたら `just e2e --only editor` も流した

## 参考

- T0055 のメモ (意図したもの)、T1669 のメモ (ドープシートの印に `data-ui-audit-overlay` を付けなかった理由)、T1668 (CurveGraph の `data-ui-audit-overlay`)
- tests/ui-audit/README.md の「重ねて描くスクロールバー」「押せる部品の重なり」、docs/ui.md (`data-ui-audit-overlay`)

## メモ

- T0063 で起票した (元の候補: T0055 (帯の端のキーの印が scrollbar の検査に毎回出る))。
- 同じ tests/ui-audit/README.md の「重ねて描くスクロールバー」を直す T0979 の後にした。
- 決めたこと: 案 A (`data-ui-audit-overlay` を scrollbar の検査でも見る)。理由:
  - 「意図してほかのもの (部品・スクロールバー) と重ねた」という意味が同じで、印の種類を増やさずに済む (B は検査ごとに印が増える)。
  - 2 つの印 (`.marker`・`.diamond`) は `position: absolute` なので、重なりの検査 (`overlaid`) からはもとから外れている。重なりの検査は
    上に描かれた側の祖先の印を見るだけなので、印を付けても重なりの検査の結果は変わらない (下の比較で、重なりは前も後も 0)。
  - C (残す) は、毎回 2 種類 (延べ 249) が出て新しい指摘を埋もれさせる。
  - 印を付けてよいのは、空きに出るのが押せる範囲の端だけで、見た目の部品が空きの中にあるもの、と tests/ui-audit/README.md に書いた。
- やったこと:
  - `lib/page-checks.js` の `underScrollbar` に `intentional(frame, item)` (省けば無し) を足し、いちばん内側の掛かる枠で印があれば数えない。
    `scrollbarAudit` は部品から枠の手前までの `data-ui-audit-overlay` を見る。`test/checks.test.mjs` にテストを 1 つ足した。
  - ParametersPanel.svelte の `button.marker` と MotionEditor.svelte の `button.diamond` に印と理由のコメントを付けた。
  - tests/ui-audit/README.md (重ねて描くスクロールバーの「意図したもの」・残る指摘、押せる部品の重なりの印の説明) と docs/ui.md (送りの確かめ方) を直した。
  - `just ui-audit --only editor --variants zoom200,narrow` (329 場面) を、直す前に `--out target/ui-audit-before` で撮り、直した後に
    `--compare` で比べた: +0 -2 ~0 (`button.diamond`・`button.marker` の scrollbar が消え、すべての検査で 0 件)。
  - `just ui-audit-check`・エディタの部品のテスト (ParametersPanel・MotionEditor)・`just ci` が通った。クラス名と並びは変えていない
    (属性を足しただけ) ので `just e2e --only editor` は流していない。
- 起票の候補: なし。

