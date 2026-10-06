---
id: T0331
title: "モーション: 終わりちょうど (t = 長さ) のイベントが、丸めでときどき発火しないのを直す"
status: in-progress
milestone: M10
crates: [ugoe-motion, docs]
depends_on: []
assignee: claude-worker-T0331
---

# T0331: モーション: 終わりちょうど (t = 長さ) のイベントが、丸めでときどき発火しないのを直す

## 目的

T0919 の e2e で、ビューアの見本の `nod` の終わりのイベント `nod-end` (t = 長さ) が再生しても発火しないことがあった (4 回続けて逃したこともある)。
`MotionPlayer::advance` は経過時間を `f64` (`elapsed`、`advance_clock` が終了時刻 `end` で切る) で、再生位置を `f32` (`time` に `dt` を足す) で持つ。
最後の刻みで `advance_clock` が返す `dt` (`(end - elapsed) as f32`) を足しても、丸めで `time` が長さに届かないまま `is_finished()` になり、
`fire_range` の `t <= to` を満たさない、と見ている。

## 受け入れ条件

- [ ] 直す前に失敗するテストがある (例: 長さを割り切れない `dt` の列 (1/60 など) で再生し、t = 長さのイベントが、終わる前のどこかの `advance` で
      ちょうど 1 回発火する。いくつかの長さ・刻みで回す)。原因が上の見立てと違えば、メモに書く
- [ ] 終わりに着いた (`elapsed` が `end` に達した) `advance` では、再生位置を長さまで進めて終わりのイベントを発火する。ループしないモーションの
      t = 長さのイベントが、刻みの丸めによらずちょうど 1 回発火する。ループするモーションの周回の終わりでも同じ性質をテストで確かめた
- [ ] 発火の範囲の決め (`(from, to]`・開始直後の `[from, to]`) を変えたなら docs/spec/motion.md と英語の公開仕様 (spec/) の対応する節を直した
- [ ] `just crate ugoe-motion`・`just wasm-check`・`just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」
- crates/ugoe-motion/src/player.rs (`advance_clock`・`MotionPlayer::advance`・`fire_range`)、docs/spec/motion.md

## メモ

- T0046 で起票した (元の候補: T0919 (モーションの最後のイベントがときどき発火しない))。
- 見つけたのは M10 の T0919 なので milestone は M10 にした (ランタイムの不具合で、UI のタスクではない)。M11 (no_std・C ABI) の範囲ではない。
