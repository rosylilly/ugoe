---
id: T0883
title: "スタジオ: 検出が遅い (値が 1 秒に数個) ときも促しの余裕を見直すよう、見直しの区切りを値の数で延ばす"
status: todo
milestone: M8
crates: [apps/studio, tests/e2e, docs]
depends_on: []
assignee:
---

# T0883: 余裕を見直す区切りを値の数で延ばす

## 目的

`apps/studio/src-tauri/src/pacing.rs` の `adjust_margin` は、1 秒 (`MARGIN_WINDOW`) の区切りに値が `MARGIN_MIN_VALUES` (10) 個以上あるときだけ、
促しの余裕 (1〜4 ms) を見直す。検出が重く値が 1 秒に約 2 個のときは一度も見直さず、スリープの遅れで遅れた値が増えても余裕は 1 ms のまま
(T0878 の `late_values_spread_over_phases` の e2e に近い形で、余裕を見直さない `fixed_margin` と数が全く同じ。遅れた値の中央値 約 20%)。
値の少ないときは、区切りを値の数で延ばして (例: 10 個たまるまで) 見直すようにする。

## 決まっていること (利用者、2026-10-11)

- 振る舞いを変えてよい (T0878 の起票の候補をそのまま起票する。判断の要らないもの)。

## 受け入れ条件

- [ ] `adjust_margin` の区切りを、1 秒がたっても値が `MARGIN_MIN_VALUES` に足りなければ値がそろうまで延ばす (古すぎる値で見直さないよう、延ばす長さに上限を
      持つ。上限は T0875 の値の古さの上限 (`MAX_LEAD_MAX_AGE`) とそろえるかを決めてメモに書く)。トラッキングを使っていない (値が来ない) ときは今どおり見直さない
- [ ] 仮の時刻のテスト: 値が 1 秒に約 2 個の形で、余裕が見直される (スリープの遅れが大きければ増える) ことのテストを先に書いて失敗させてから直した。
      `late_values_spread_over_phases` の e2e に近い形で、直す前と後の遅れた値の割合 (最小・中央値・最大) を比べてメモに書き、上限を直した
      (`fixed_margin` との比較の数が変わる)。落ち着いた計算機の形と、値の多い (1 秒に 30 個以上) 形の結果は変わらない
- [ ] docs/studio.md#入力 (余裕の見直しの説明と T0878 の分布) を直した
- [ ] `just e2e --only studio` が通る (「トラッキングの待ち」の e2e の上限を変えたらその理由も)
- [ ] `just ci` が通る

## 参考

- T0878 のメモ (起票の候補・分布の表)、T0867・T0868・T0875、`apps/studio/src-tauri/src/pacing.rs` (`adjust_margin`・`MARGIN_*`・`late_values_spread_over_phases`・`fixed_margin`)、
  docs/studio.md#入力・#刻みの速さと正確さ、tests/e2e/README.md (長く流す)

## メモ

- T0089 で起票した (元の候補: T0878 (検出が遅い (値が 1 秒に 2 個ほど) とき `adjust_margin` が余裕を一度も見直さない。値の数で見直しの窓を延ばす。
  `late_values_spread_over_phases` の e2e に近い形で前後を比べる))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。
- 値の数が少ない区切りでは、遅れた値の割合 (5% を超えたら増やす) が 1 個で大きく動く (10 個で 1 個遅れると 10%)。延ばしても足りなければ、
  判定の割合か数え方を変える案もメモに書く。
