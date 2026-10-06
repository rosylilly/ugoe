---
id: T0331
title: "モーション: 終わりちょうど (t = 長さ) のイベントが、丸めでときどき発火しないのを直す"
status: done
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

- [x] 直す前に失敗するテストがある (例: 長さを割り切れない `dt` の列 (1/60 など) で再生し、t = 長さのイベントが、終わる前のどこかの `advance` で
      ちょうど 1 回発火する。いくつかの長さ・刻みで回す)。原因が上の見立てと違えば、メモに書く
- [x] 終わりに着いた (`elapsed` が `end` に達した) `advance` では、再生位置を長さまで進めて終わりのイベントを発火する。ループしないモーションの
      t = 長さのイベントが、刻みの丸めによらずちょうど 1 回発火する。ループするモーションの周回の終わりでも同じ性質をテストで確かめた
- [x] 発火の範囲の決め (`(from, to]`・開始直後の `[from, to]`) を変えたなら docs/spec/motion.md と英語の公開仕様 (spec/) の対応する節を直した
- [x] `just crate ugoe-motion`・`just wasm-check`・`just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」
- crates/ugoe-motion/src/player.rs (`advance_clock`・`MotionPlayer::advance`・`fire_range`)、docs/spec/motion.md

## メモ

- T0046 で起票した (元の候補: T0919 (モーションの最後のイベントがときどき発火しない))。
- 見つけたのは M10 の T0919 なので milestone は M10 にした (ランタイムの不具合で、UI のタスクではない)。M11 (no_std・C ABI) の範囲ではない。
- 結果 (T0331):
  - 見立てどおりだった。`end_event_fires_once_regardless_of_rounding` (長さ 0.7〜10.3 の 8 通り × 刻み 1/60・1/30・1/144・0.1・0.007・0.0333) は、
    直す前は長さ 0.7・刻み 1/60 で t = 長さのイベントが 1 回も発火しないまま `is_finished()` になって失敗した。
  - 直し方: ループしないモーションの再生位置を、`time + dt` ではなく `f64` の経過時間 `min(elapsed, duration)` から求める
    (`crates/ugoe-motion/src/player.rs` の `MotionPlayer::advance`)。終わりに達した `advance` では `elapsed == end == duration` なので、
    再生位置はちょうど長さになり、`(from, to]` の範囲にイベントが入る。`stop()` で早く終わる場合も再生位置は終了時刻になる。
  - 発火の範囲 (`(from, to]`・開始直後の `[from, to]`) は変えていない。再生位置の求め方を変えたので、docs/spec/motion.md の「再生とフェード」と
    英語の公開仕様 spec/motion.md の Motion playback の擬似コード (`to = min(elapsed, max(duration, 0))`) と説明を直した。
  - ループするモーションの周回の終わりは `loop_end_event_fires_once_per_lap_regardless_of_rounding` (長さ 6 通り × 刻み 6 通りで 9 周) で、
    t = 長さのイベントが周回ごとにちょうど 1 回発火することを確かめた (直す前から通る。周回の境目は `f32` の比較 `to < duration` で決まり、
    丸めで届かなくても次のフレームで発火するので逃さない)。
  - やらなかったこと: ループするモーションを `stop()` して、終了時刻がちょうど周回の終わりに重なる場合は、`f32` の再生位置の丸めで
    t = 長さのイベントを逃すことがある (長さ 0.7・刻み 1/60 で開始直後に止め、`fade_out` = 長さにすると再現する)。
    直すには周回の始まりを `f64` の経過時間で持つなど、ループの再生位置の持ち方を変える必要があり、普通は周回の途中で終わるので範囲外にした。
  - `just crate ugoe-motion`・`just wasm-check`・`just ci` が通った。
- 起票の候補: 止めたループするモーションの終了時刻が周回の終わりに重なるとき、t = 長さのイベントを丸めで逃すことがある
  (再生位置を周回の始まりの経過時間 (`f64`) からの差で持てば直る。巨大な `dt` での桁落ちに注意)。
