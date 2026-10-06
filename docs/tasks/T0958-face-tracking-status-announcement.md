---
id: T0958
title: "フェイストラッキング: 顔が見つからないなどの状態を、揺れを抑えて読み上げで知らせるか決める (判断が要る)"
status: done
milestone: M10
crates: [packages/face-tracking, docs]
depends_on: []
assignee: claude-worker-T0958
---

# T0958: フェイストラッキング: 顔が見つからないなどの状態を、揺れを抑えて読み上げで知らせるか決める (判断が要る)

## 目的

トラッキングのパネル (`packages/face-tracking/src/TrackingPanel.svelte`) の状態の文は、fps・再生の位置と同じ文で毎フレーム変わるので
live region にしていない (T0918)。そのため、顔が見つからない・体が見つからないなどの状態の変化は、スクリーンリーダーの利用者に伝わらない (T0918 の候補)。

**判断が要る:** 案:

- A. 状態 (顔が見つからない・体が見つからない・止まった など) だけを fps と分け、変わってから一定の時間 (例 1 秒) 続いたら `announce()` で知らせる。
- B. 状態だけを分けた `role="status"` の要素に置き、文が変わったときだけ読ませる (揺れは時間の閾値で抑える)。
- C. 知らせない (状態は Tab で行って読める。docs/ui.md に理由を書く)。

推奨は、トラッキング中に顔が出入りする頻度 (録画の再生で数える) と、配信中の読み上げの邪魔にならないかを見てから書く。

## 受け入れ条件

- [x] 案と推奨をメモに書き、利用者の判断を得た
- [x] 決めた形で直し、揺れ (短い間の出入り) では知らせないことを部品か状態のテストで確かめた (C なら文書だけ)
- [x] docs/ui.md (読み上げで知らせるもの) を直した
- [x] ビューアとスタジオの e2e (トラッキングの手順があれば) と `just ci` が通る

## 参考

- T0918 のメモ (起票の候補)
- packages/face-tracking/src/TrackingPanel.svelte・tracker.svelte.ts、packages/ui (`announce()`)、docs/ui.md

## メモ

- T0043 で起票した (元の候補: T0918 (フェイストラッキングの状態の読み上げ))。
- 判断 (2026-10-06): 利用者が案 B を選んだ (状態だけを分けた `role="status"` の要素に置き、変わってから約 1 秒続いたときだけ文を変える)。
- やったこと (案 B):
  - `packages/face-tracking/src/status.ts` を足した。`trackingCondition()` (FaceTracker の値 → 準備中・止まった・顔が無い・体が無い・
    トラッキング中・記録を再生中・止めている) と、`SettledValue` (入った値が `delayMs` 続いたときだけ見せる値を変える。時刻は引数。
    止めた (`idle`) ときは待たずに消す)。待つ時間は `CONDITION_SETTLE_MS` = 1000。
  - `TrackingPanel.svelte`: 見出しの横の文を、`role="status"` の状態の文 (描いたときから置く) と、live region の外の fps・再生の位置に分けた。
    状態の文は `SettledValue` を通し、待っている間は `setTimeout` で決まる時刻に見直す。見出しの幅を空けるため、トラッキング中・記録を再生中の文は
    `.ugoe-visually-hidden` で見せずに読ませる (見た目は前と同じく fps・再生の位置だけ)。警告の色は顔・体が無い・止まったのとき。
  - 文言を足した (`status.tracking`・`status.replaying`・`status.stopped`。en・ja)。
  - テスト: `status.test.ts` (状態の決め方、1 秒続いたら変わる、0.1 秒ずつの出入りと 0.9 秒の外れでは変わらない、途中で別の値に変わったら待ち直す、
    止めたら待たずに消す)。
  - docs/ui.md#状態の変化の知らせ に、その場の `role="status"` で知らせるものとしてトラッキングの状態を書き、確かめ方の表にテストを足した。README の一覧にも足した。
- 確かめたこと: `pnpm --filter @ugoe/face-tracking test`・`check`、`just e2e --only viewer` (6 件)・`just e2e --only studio` (13 件)・`just e2e --only editor` (15 件。プレビューも同じパネルを使う) が通る。
  rebase の後の `just ci` が通る。
- やらなかったこと: トラッキング中に顔が出入りする頻度の計測 (判断が済んだため)。実際のスクリーンリーダー (NVDA・VoiceOver) で読まれるかは
  確かめていない (手作業の確かめ U03 の範囲)。
- 気づいたこと: 待つ間は見せる文も前のまま (準備中から動き出して 1 秒弱は「準備中…」と fps が並ぶ)。見た目だけ即座に変える形にすると、
  同じ文が見た目と読み上げで食い違うので、見た目も読み上げに合わせた。
- 起票の候補: なし。
