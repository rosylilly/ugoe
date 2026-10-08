---
id: T0863
title: "スタジオ: 「フレームレートと負荷」に、トラッキングの値が届いてから刻みで使われるまでの待ち (平均と最大) を出す"
status: in-progress
milestone: M8
crates: [apps/studio, docs, tests/ui-audit, tests/e2e]
depends_on: [T0862]
assignee: claude-worker-T0863
---

# T0863: スタジオ: トラッキングの待ちを負荷の表示に出す

## 目的

T0854 で、操作ウィンドウへの入力の処理の促しを刻みの前に寄せ、トラッキングの値が刻みの約 1 ms (`LEAD_MARGIN`) 前に届くようにした。
待ちはテスト (`ticker.rs` の `measure_tracking_wait`、偽の操作ウィンドウ) の中でしか測っておらず、実機 (実際のカメラと MediaPipe・Windows・
macOS のスリープの粒度) でどれだけ縮んだか・刻みに遅れる値がどれだけあるかを見る手段が無い。「フレームレートと負荷」に出して、
実機で確かめられるようにする (T0864 で `LEAD_MARGIN` を決める材料にもなる)。

## 受け入れ条件

- [ ] 刻みのスレッドが、トラッキングの値が届いてからその値を使う刻みまでの待ちを、直近の約 1 秒で集める (平均・最大と、
      刻みに遅れて 1 刻み以上待った値の割合)。確保は毎刻み増やさない。`pacing.rs` は時計とスレッドを持たない形を保ち、集め方は単体テストで確かめた
- [ ] 状態の通知 (`StatusMessage`。`measuredRate` などと同じ所) に載せ、操作ウィンドウの「フレームレートと負荷」に出す。トラッキングの値が
      届いていない間は出さない (「—」)。`apps/studio/src/lib/ipc.ts` 相当の型と、偽のホスト (tests/ui-audit・tests/e2e) の形をそろえた
- [ ] 注意は出さない (待ちは目安。注意を出すかは T0864 で測ってから決める)。表示の文言は @ugoe/i18n の日英に足した
- [ ] docs/studio.md#負荷の表示 の表と #入力 (待ちを測る手段) を直した。docs/manual-check.md に、実機で待ちを見る手順を足した
      (T0854 の S の項か新しい項。カメラの 30・60 fps と刻み 30・60・120)
- [ ] 画面に値を足したので `just e2e --only studio` と `just ui-audit --only studio` を流した
- [ ] `just ci` が通る

## 参考

- T0854 のメモ (測ったもの・起票の候補)、apps/studio/src-tauri/src/pacing.rs (`Pacer::arrived`)・ticker.rs (`measure_tracking_wait`)
- docs/studio.md#入力・#刻みの速さと正確さ・#負荷の表示、apps/studio/src/LoadPanel.svelte

## メモ

- T0077 で起票した (元の候補: T0854 (値が届いてから刻みで使われるまでの待ちを出す))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T0862 と同じ LoadPanel.svelte を触るので、T0862 の後にした。
