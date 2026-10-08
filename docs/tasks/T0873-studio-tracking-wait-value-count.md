---
id: T0873
title: "スタジオ: 「トラッキングの待ち」に値の数を載せ、e2e で遅れた値の合計と割合を出す"
status: done
milestone: M8
crates: [apps/studio, tests/e2e, docs]
depends_on: []
assignee: claude-worker-T0873
---

# T0873: スタジオ: 「トラッキングの待ち」に値の数を載せ、e2e で遅れた値の合計と割合を出す

## 目的

T0871 で、e2e (tests/e2e/specs/studio.test.mjs の T0812 の手順) は「フレームレートと負荷」の「トラッキングの待ち」を約 1 秒ごとに読み、
`tracking.json` の `trackingWaitSummary` に残すようにした。ただし表示の文 (平均・最大・遅れた割合・余裕) に値の数が出ないので、
遅れた割合は各回の割合の平均になっている (値が 2〜3 個しか無い回も、10 個ある回も同じ重み。1 回の表示が 0・33・50% と大きく揺れる)。
バックエンドの状態 (`get_status` の `trackingWait`) に約 1 秒の値の数と遅れた値の数を足し、e2e は値の数で重みを付けた合計と割合を出す。

## 受け入れ条件

- [x] `pacing.rs` の `TrackingWait` (`get_status` の `trackingWait`) に、約 1 秒に届いた値の数と遅れた値の数を足した (`WaitMeter` は既に数えている。
      刻みのたびの確保を増やさない)。`apps/studio/src/lib/host.ts` の型と、偽のホスト (e2e・ui-audit・`load.test.ts`・`status.test.ts` の値) を合わせた
- [x] 画面の表示は変えない (割合のまま。数を出すなら文言を日英で足し、`@ugoe/i18n` の検査を通す)。変えたら `just ui-audit --only studio` が通る
- [x] e2e が、表示の文ではなく状態 (`get_status`) から約 1 秒ごとの値の数と遅れた値の数を集め、`tracking.json` に合計 (値の数・遅れた値の数) と
      値の数で重みを付けた割合を残す。今の各回の割合の平均も残してよい。判定 (閾値) は足さない (T0871 と同じ理由。値は計算機の重さで揺れる)
- [x] tests/e2e/README.md の「結果」の `tracking.json` の欄と、docs/studio.md#負荷の表示 (状態に値の数を載せること) を直した
- [x] `just e2e --only studio` が通り、3 回分の合計と割合をメモに書いた
- [x] `just ci` が通る

## 参考

- T0871 のメモ (やったこと・e2e の表・起票の候補)、T0863・T0867・T0868 のメモ、docs/studio.md#入力・#負荷の表示
- apps/studio/src-tauri/src/pacing.rs (`WaitMeter`・`TrackingWait`)、apps/studio/src-tauri/src/lib.rs (`get_status`)、apps/studio/src/lib/host.ts、
  tests/e2e/specs/studio.test.mjs (`TRACKING_WAIT`・`trackingWaits`・`trackingWaitSummary`)、tests/ui-audit/lib/studio-host.mjs

## メモ

- T0081 で起票した (元の候補: T0871 (トラッキングの待ちを値の数が分かる形で集め、遅れた値の合計を出す。status の trackingWait に値の数))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- 外部 API (docs/studio-api.md) は負荷の表示を出していないので触らない。
- T0875 (案 A・B を選んだ場合) と同じ `pacing.rs` を触るが、こちらは集計の欄だけなので並べて進めてよい。
- git stash は使わない。

### やったこと (claude-worker-T0873)

- `pacing.rs` の `TrackingWait` に `values` (約 1 秒に届いた値の数) と `late_values` (遅れた値の数) を足した。`WaitMeter` が数えていた
  `values`・`late` をそのまま載せるだけで、確保は増えない。`wait_meter_collects_a_second_of_waits` と lib.rs の `Monitor::Status` の
  JSON のテストに数を足した。
- 注: タスクの文の `get_status` というコマンドは無く、状態は操作ウィンドウへの監視のチャンネル (`Monitor::Status`) で届く。
  e2e が状態を読めるよう、`Control.svelte` が届いた `status` を `window.__UGOE_STUDIO_STATUS__` に置くようにした (`global.d.ts` に型、
  apps/studio/README.md の「ヘッドレスのブラウザで確かめる」に記載。`window.__UGOE_TRACKER__` と同じ形)。
- `host.ts` の `TrackingWait` に `values`・`lateValues`。偽のホストの値 (ui-audit の studio-host.mjs・`load.test.ts`・`LoadPanel.svelte.test.ts`) を合わせた
  (`status.test.ts` は `trackingWait: null` なので変更なし)。画面の表示は変えていない (`just ui-audit --only studio` は通った)。
- e2e (T0812 の手順): 6 秒から 13 秒まで、繰り返し (約 250 ms ごと) のたびに状態の `trackingWait` を読み、前と同じ集計は除いて
  `trackingWaits` (`atMs`・`wait`) に残す。`trackingWaitSummary` に `values`・`lateValues` の合計と値の数で重みを付けた `latePercent` を足し、
  各回の割合の平均 `meanLatePercent` と最大も残した。判定は足していない (手順の終わりに表示の文が出ることだけを確かめる)。
  表示の文を読む `parseTrackingWait` は使わなくなったので消した。
- tests/e2e/README.md の `tracking.json` の欄と、docs/studio.md#負荷の表示 を直した。

### e2e の結果 (`just e2e --only studio`、WSL2、3 回とも通った)

| 回 | 集計の回数 | 値の数 | 遅れた値 | 重み付きの割合 | 各回の割合の平均 | 最大 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 7 | 17 | 2 | 11.76% | 11.9% | 50% |
| 2 | 7 | 18 | 5 | 27.78% | 23.81% | 100% |
| 3 | 7 | 17 | 0 | 0% | 0% | 0% |

読んだ時刻の間隔は約 0.4〜1.3 秒で、約 1 秒の集計を取りこぼしていない (7 秒で 7 回)。合計しても値は 17〜18 個しか無く、回ごとに 0〜28% と揺れる。

### やらなかったこと

- 値が届かない約 1 秒が続いたとき、同じ `null` の集計が続くので `unread` は 1 と数える (前と同じ集計を除くため。今の e2e では 0)。

### 起票の候補

- e2e の重い検出の手順は 7 秒で値が 17 個ほどしか集まらず、合計の割合も 0〜28% と揺れる。前と後を比べるには、集める時間を延ばす
  (例えば 20 秒) か、`pacing.rs` を変えたときだけ長く流す設定 (環境変数) を足す。

