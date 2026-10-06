---
id: T0332
title: "モーション: ループするモーションを stop() し、止まる時刻が周回の終わりに重なると、t = 長さのイベントを丸めで逃す"
status: todo
milestone: M10
crates: [ugoe-motion, docs, spec]
depends_on: []
assignee:
---

# T0332: モーション: ループするモーションを stop() し、止まる時刻が周回の終わりに重なると、t = 長さのイベントを丸めで逃す

## 目的

T0331 でループしないモーションの再生位置を f64 の経過時間から求めるようにしたが、ループするモーションを `stop()` し、
フェードアウトの終わりがちょうど周回の終わりに重なる場合は、t = 長さのイベントをまだ丸めで逃すことがある (T0331 の候補)。
再現: 長さ 0.7・刻み 1/60 で開始直後に止め、`fade_out` = 長さ。

## 受け入れ条件

- [ ] 直す前に失敗するテストで再現した (`ugoe_model::fixtures` か、テストの中で作ったモーション)
- [ ] 周回の始まりを f64 の経過時間で持ち、その差を再生位置にするなどで直した。巨大な `dt`・長い再生での桁落ちと、時間のループの上限 (AGENTS.md) に気をつけ、テストで確かめた
- [ ] T0331 のテスト (`end_event_fires_once_regardless_of_rounding`・`loop_end_event_fires_once_per_lap_regardless_of_rounding`) が通る
- [ ] 再生位置の求め方が変わるなら docs/spec/motion.md と英語の spec/motion.md を直した
- [ ] `just crate ugoe-motion`・`just wasm-check`・`just ci` が通る

## 参考

- T0331 のメモ、crates/ugoe-motion/src/player.rs (`MotionPlayer::advance`)

## メモ

- T0331 のワーカーの報告から起票した (2026-10-06)。
