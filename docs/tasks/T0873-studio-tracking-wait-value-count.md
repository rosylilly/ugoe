---
id: T0873
title: "スタジオ: 「トラッキングの待ち」に値の数を載せ、e2e で遅れた値の合計と割合を出す"
status: todo
milestone: M8
crates: [apps/studio, tests/e2e, docs]
depends_on: []
assignee:
---

# T0873: スタジオ: 「トラッキングの待ち」に値の数を載せ、e2e で遅れた値の合計と割合を出す

## 目的

T0871 で、e2e (tests/e2e/specs/studio.test.mjs の T0812 の手順) は「フレームレートと負荷」の「トラッキングの待ち」を約 1 秒ごとに読み、
`tracking.json` の `trackingWaitSummary` に残すようにした。ただし表示の文 (平均・最大・遅れた割合・余裕) に値の数が出ないので、
遅れた割合は各回の割合の平均になっている (値が 2〜3 個しか無い回も、10 個ある回も同じ重み。1 回の表示が 0・33・50% と大きく揺れる)。
バックエンドの状態 (`get_status` の `trackingWait`) に約 1 秒の値の数と遅れた値の数を足し、e2e は値の数で重みを付けた合計と割合を出す。

## 受け入れ条件

- [ ] `pacing.rs` の `TrackingWait` (`get_status` の `trackingWait`) に、約 1 秒に届いた値の数と遅れた値の数を足した (`WaitMeter` は既に数えている。
      刻みのたびの確保を増やさない)。`apps/studio/src/lib/host.ts` の型と、偽のホスト (e2e・ui-audit・`load.test.ts`・`status.test.ts` の値) を合わせた
- [ ] 画面の表示は変えない (割合のまま。数を出すなら文言を日英で足し、`@ugoe/i18n` の検査を通す)。変えたら `just ui-audit --only studio` が通る
- [ ] e2e が、表示の文ではなく状態 (`get_status`) から約 1 秒ごとの値の数と遅れた値の数を集め、`tracking.json` に合計 (値の数・遅れた値の数) と
      値の数で重みを付けた割合を残す。今の各回の割合の平均も残してよい。判定 (閾値) は足さない (T0871 と同じ理由。値は計算機の重さで揺れる)
- [ ] tests/e2e/README.md の「結果」の `tracking.json` の欄と、docs/studio.md#負荷の表示 (状態に値の数を載せること) を直した
- [ ] `just e2e --only studio` が通り、3 回分の合計と割合をメモに書いた
- [ ] `just ci` が通る

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
