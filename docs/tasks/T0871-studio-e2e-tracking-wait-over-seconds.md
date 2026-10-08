---
id: T0871
title: "スタジオの e2e: T0812 の手順で「トラッキングの待ち」を数秒分集め、遅れた値の割合を前後で比べられるようにする"
status: in-progress
milestone: M8
crates: [tests/e2e, docs]
depends_on: []
assignee: claude-worker-T0871
---

# T0871: スタジオの e2e: T0812 の手順で「トラッキングの待ち」を数秒分集める

## 目的

e2e の「tracking under heavy detection reaches the control and the output (T0812)」(tests/e2e/specs/studio.test.mjs) は、
「フレームレートと負荷」の「トラッキングの待ち」(Tracking wait) を手順の終わりに 1 回だけ読み、`tracking.json` に残す。
この表示は直近の約 1 秒の値 (検出 200 ms・カメラ 2 fps では 2〜3 個) の集計なので、1 つ遅れるだけで 0〜67% と揺れ、
T0868 の前と後の差 (仮の時刻のテストでは 18% → 6%) が e2e では見えなかった (T0868 のメモの「e2e」)。
数秒分の表示を集めて残し、変更の前と後を e2e の出力で比べられるようにする。

## 受け入れ条件

- [ ] 手順の間 (少なくとも値が 10 個そろう約 5 秒の後の数秒) に「トラッキングの待ち」を 1 秒ほどの間隔で数回読み、各回の平均・最大・遅れた割合・余裕を
      `tracking.json` に配列で残した (文の形が変わったら壊れないよう、今の正規表現で読み取る)。集めた割合の平均 (または遅れた値の合計) も出す
- [ ] 判定は今と同じ (表示が出ること) で、割合の値で落とさない (揺れて落ちる検査にしない)。理由をテストのコメントに書いた
- [ ] 手順が長くなるなら、増える時間をメモに書いた (目安: 今の 10 秒に数秒足すまで)
- [ ] `just e2e --only studio` を前の `pacing.rs` (T0868 の前。一時的なコミットか別の worktree で) と今で 2〜3 回ずつ流し、集めた割合の前後をメモに書いた
- [ ] tests/e2e/README.md (出力の説明) を直した
- [ ] `just ci` が通る

## 参考

- T0868 のメモ (e2e の測った値と起票の候補)、T0863・T0867 のメモ
- tests/e2e/specs/studio.test.mjs (`trackingWait`・`tracking.json`)、docs/studio.md#入力、docs/manual-check.md の S36

## メモ

- T0080 で起票した (元の候補: T0868 (e2e の T0812 の手順で「Tracking wait」を数秒分集め、遅れた値の割合を前後で比べられるようにする))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- git stash は使わない (前と後は一時的なコミットか別の worktree で比べる)。
