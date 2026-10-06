---
id: T0332
title: "モーション: ループするモーションを stop() し、止まる時刻が周回の終わりに重なると、t = 長さのイベントを丸めで逃す"
status: done
milestone: M10
crates: [ugoe-motion, docs, spec]
depends_on: []
assignee: claude-worker-T0332
---

# T0332: モーション: ループするモーションを stop() し、止まる時刻が周回の終わりに重なると、t = 長さのイベントを丸めで逃す

## 目的

T0331 でループしないモーションの再生位置を f64 の経過時間から求めるようにしたが、ループするモーションを `stop()` し、
フェードアウトの終わりがちょうど周回の終わりに重なる場合は、t = 長さのイベントをまだ丸めで逃すことがある (T0331 の候補)。
再現: 長さ 0.7・刻み 1/60 で開始直後に止め、`fade_out` = 長さ。

## 受け入れ条件

- [x] 直す前に失敗するテストで再現した (`ugoe_model::fixtures` か、テストの中で作ったモーション)
- [x] 周回の始まりを f64 の経過時間で持ち、その差を再生位置にするなどで直した。巨大な `dt`・長い再生での桁落ちと、時間のループの上限 (AGENTS.md) に気をつけ、テストで確かめた
- [x] T0331 のテスト (`end_event_fires_once_regardless_of_rounding`・`loop_end_event_fires_once_per_lap_regardless_of_rounding`) が通る
- [x] 再生位置の求め方が変わるなら docs/spec/motion.md と英語の spec/motion.md を直した
- [x] `just crate ugoe-motion`・`just wasm-check`・`just ci` が通る

## 参考

- T0331 のメモ、crates/ugoe-motion/src/player.rs (`MotionPlayer::advance`)

## メモ

- T0331 のワーカーの報告から起票した (2026-10-06)。
- 結果 (T0332):
  - 再現: `stopped_loop_end_event_fires_when_end_meets_lap_end` (長さ 8 通り × 刻み 6 通り、開始直後に `stop()`・`fade_out` = 長さ) は、
    直す前は長さ 0.7・刻み 1/60 で t = 長さのイベントが 1 回も発火しないまま終わって失敗した。
  - 直し方: `MotionPlayer` に今の周回が始まった経過時間 `lap_start` (`f64`) を持たせ、ループするモーションの再生位置を
    `max(elapsed - lap_start, 0)` から求める。周回の終わりに達したら `lap_start += duration` で折り返す
    (`crates/ugoe-motion/src/player.rs` の `MotionPlayer::advance`)。終了時刻 `elapsed + fade_out` と周回の終わり `lap_start + duration` が
    `f64` で同じ値になるので、終わりのフレームで周回の終わりに届き、t = 長さのイベントが発火する。
  - 周回の上限 (`MAX_LAPS_PER_ADVANCE`) は保った。上限を超えたら `lap_start` を `elapsed - (elapsed - lap_start) mod duration` に合わせ、
    桁落ちで差が長さに届くときは `lap_start = elapsed`・再生位置 0 にする (次の `advance` で周回を回し続けない)。
    `looping_advance_terminates_for_extreme_values` を、巨大な `dt` の後の `advance` も周回の上限内で終わり、再生位置が `[0, duration)` に収まることまで確かめるように広げ、
    `looping_keeps_playing_after_a_long_run` (約 12 日進めた後も 1/60 ずつ進み、周回ごとに発火する) を足した。
  - 振る舞いの違い: 経過時間が極端に大きい (1/60 秒が `f64` に足せなくなる 10^14 秒ほど。巨大な `dt` を 1 回渡したときなど) と、
    その後の再生位置は進まなくなる (前は `f32` の再生位置が独立に進んだ)。経過時間のフェードと終了時刻は前から同じ状態なので許容した。仕様に書いた。
  - docs/spec/motion.md の「再生とフェード」と spec/motion.md の Motion playback (擬似コード・説明・参考実装の注記) を直した。
  - `just crate ugoe-motion`・`just wasm-check`・`just ci` が通った。
- 起票の候補: なし。
