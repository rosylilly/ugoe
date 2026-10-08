---
id: T0872
title: "スタジオ: トラッキングの値が 1 秒に 1 個より少ないときも促しを寄せ続ける (戻すまでの時間を値の間隔で決める)"
status: todo
milestone: M8
crates: [apps/studio, docs]
depends_on: []
assignee:
---

# T0872: スタジオ: 値が 1 秒に 1 個より少ないときも促しを寄せ続ける

## 目的

`Pacer` は、直近の `LEAD_WINDOW` (1 秒) に値が届いていなければ促しを寄せるのをやめ (`aim` が `None`)、促しを刻みの直後に戻す
(apps/studio/src-tauri/src/pacing.rs の `aim`)。T0868 で見積もりに使う値は 10 個・5 秒まで残すようにしたが、この「戻す」判定は 1 秒のままなので、
検出がとても重く値が 1 秒に 1 個より少ない (例: 1.5 秒に 1 個) ときは、値が届くたびに寄せては次の値の前に戻すのを繰り返し、
寄せた促しの効果が出ない (T0868 のメモの「やらなかったこと」)。戻すまでの時間を値の間隔で決め、とても重い検出でも寄せ続ける。

## 受け入れ条件

- [ ] 促しを戻すまでの時間を、直近の値の間隔から決めた (例: 直近の値の間隔の中央値の 2 倍か `LEAD_WINDOW` の大きいほう、上限は `LEAD_MAX_AGE`)。
      決めた式と理由を `pacing.rs` のコメントと docs/studio.md#入力 に書いた。刻みのたびの確保を増やさない
- [ ] 仮の時刻のテスト (`simulate_with` と T0868 の `Detector`) で、値が 1 秒に 1 個より少ない場合 (例: 検出 1,200 ms と 2,000 ms、刻み 30・60・120) に
      寄せたままになることと、遅れた値の割合が前より下がるか変わらないことを確かめた。前と後の数をメモに書いた
- [ ] トラッキングが止まった (値が来なくなった) ときは、今と同じく上限の時間の後に促しを戻す (テストで確かめた)
- [ ] 値が多い場合 (T0864・T0868 のテスト: `margin_grows_when_sleeps_wake_late`・`widely_varying_detection_does_not_leave_many_values_late`・
      `few_values_from_slow_detection_are_rarely_late`・`old_leads_are_dropped_when_detection_changes`) の数が変わらないか良くなる
- [ ] `just e2e --only studio` が通る
- [ ] `just ci` が通る

## 参考

- T0868 のメモ (やらなかったこと・起票の候補)、T0864・T0867 のメモ、docs/studio.md#入力・#刻みの速さと正確さ
- apps/studio/src-tauri の `pacing.rs` (`Pacer::aim`・`LEAD_WINDOW`・`MIN_LEADS`・`LEAD_MAX_AGE`・`prune_leads`)

## メモ

- T0080 で起票した (元の候補: T0868 (値が 1 秒に 1 個より少ないとき、促しを刻みの直後に戻すまでの時間を値の間隔で決める))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- 実際のカメラと MediaPipe、Windows・macOS での値は S36 で利用者が見る (作業者の環境では確かめられない)。
