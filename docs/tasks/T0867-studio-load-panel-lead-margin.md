---
id: T0867
title: "スタジオ: 「トラッキングの待ち」に今の促しの余裕 (1〜4 ms) も出す"
status: done
milestone: M8
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0867
---

# T0867: スタジオ: 「トラッキングの待ち」に今の促しの余裕も出す

## 目的

T0864 で、トラッキングの促しの余裕 (`Pacer` の `margin`) を 1〜4 ms の間で自分で合わせるようにしたが、今の余裕は画面から見えない。
docs/manual-check.md の S36 で、Windows・macOS で余裕が上限 (4 ms) に張り付いているか (スリープの揺れが余裕で吸収しきれていないか) を
見られるように、「フレームレートと負荷」の「トラッキングの待ち」の行に今の余裕も出す。

## 受け入れ条件

- [x] 刻みのスレッドが今の余裕を負荷の報告に載せる (`pacing.rs` は時計とスレッドを持たない形のまま。仮の時刻のテストで、揺れたときに増えた余裕が報告に出る)
- [x] 「トラッキングの待ち」の行に余裕を出す (例: 「平均 1.2 ms、最大 3 ms、遅れ 0%、余裕 1.5 ms」)。促しを寄せていないとき (値が無い・「—」) の表示を決めて書いた。
      文言は日英、区切りは T0862 の `load.separator`
- [x] LoadPanel.svelte のテストで表示と読み上げの区切りを確かめた
- [x] docs/studio.md#負荷の表示 の表、docs/manual/studio.md の「トラッキングの待ち」の説明、docs/manual-check.md の S36 (余裕も書く) を書き直した
- [x] `just e2e --only studio` と `just ui-audit --only studio` が通る (画像が変わったら撮り直す)
- [x] `just ci` が通る

## 参考

- T0863・T0864 のメモ、docs/studio.md#入力・#負荷の表示・#刻みの速さと正確さ、docs/manual-check.md の S36
- apps/studio/src-tauri の `pacing.rs`、apps/studio/src の LoadPanel.svelte

## メモ

- T0078 で起票した (元の候補: T0864 (「トラッキングの待ち」に今の余裕 (1〜4 ms) も出す。S36 で Windows・macOS の余裕が上限に張り付いているかを見る))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T0868 も `pacing.rs` を触るので、T0868 はこのタスクの後にする。
- Windows・macOS の実機の値は S36 で利用者が見る (作業者の環境では確かめられない)。
- やったこと:
  - `pacing.rs`: `Pacer::tick` が返す `Ticked` に今の余裕 (`margin`。見直した後の値) を足した。促しを寄せていない (`aim` が無い。直近の約 1 秒に
    番号の付いた値が届いていない) ときは `None`。`WaitMeter` は集め終えた刻みの余裕を `TrackingWait::margin_ms` (JSON は `marginMs`、無しは `null`) に載せる。
    時計とスレッドは持たない形のまま。仮の時刻のテスト: `margin_grows_when_sleeps_wake_late` で、スリープが揺れて増えた余裕 (1 ms より大きく 4 ms 以下) が
    `WaitMeter` の報告に出ること、`widely_varying_detection_does_not_leave_many_values_late` で 1 ms のままと報告されること、
    `tick_reports_the_value_it_used_and_late_values` で寄せる前は `None`・寄せた後は `Some(1 ms)`、`wait_meter_collects_a_second_of_waits` で
    集め終えた刻みの値を出すこと・寄せていなければ無しになることを確かめた。`simulate_with` は各刻みの `Ticked::margin` が `Pacer` の余裕と等しいことも確かめる。
  - LoadPanel.svelte: 「トラッキングの待ち」の行の終わりに「余裕 1.5 ms」(英語は「margin 1.5 ms」) を `load.separator` でつないで出す。
    余裕が無し (`null`) のときは「余裕 —」を見せて読み上げからは隠し、読み上げには「余裕なし (促しを寄せていません)」を `.ugoe-visually-hidden` で読ませる
    (行ごとの「—」と同じ形)。値が届いていない (行ごと「—」) ときは今までどおり。
  - テスト・偽のホスト: LoadPanel.svelte.test.ts (値と区切り、無しの時の見える文と読み上げの文)、`lib.rs` の `Monitor::Status` の JSON、
    `tests/ui-audit/lib/studio-host.mjs` の偽の報告に `marginMs: 1.5`、`tests/e2e/specs/studio.test.mjs` の「トラッキングの待ち」の形に「, margin … ms」を足した
    (`crates:` の外の tests/ だが、文言の変更に合わせる小さな変更)。`ticker.rs` の `measure_tracking_wait` の出力にも余裕を足した。
  - 文書: docs/studio.md (入力の「待ちを実機で見る」に余裕の報告、負荷の表示の表)、docs/manual/studio.md (余裕の説明と 4 ms に張り付く意味、画像の説明)、
    docs/manual-check.md の S36 (余裕を書く・上限に張り付いているかを見る・トラッキング中の「余裕 —」は NG)。
  - `just e2e --only studio` 通過 (この機械の e2e の測定は「Average 8.38 ms, max 12.66 ms, 66.7% missed their tick, margin 1 ms」。混んだ機械)。
    `just ui-audit --only studio` 通過 (183 場面、指摘 0)。studio-load.png を `just e2e --only studio --update-images` で撮り直した
    (1 回目は機械が混んでいて起動の WebDriver が時間切れ。2 回目で通過)。ほかのスタジオの画像は撮り直しの揺れだけだったので戻した。
    写っている GPU の名前「Apple GPU」は WebKitGTK が伏せた名前で、前の画像と同じ (撮った機械に固有のものは写っていない)。
  - `just ci` 通過。
- やらなかったこと: Windows・macOS の実機で余裕の値を見ること (S36 で利用者が見る)。
- 起票の候補: なし。
