---
id: T0158
title: ループするモーションが無限大や巨大な dt で止まらない
status: done
milestone: M9
crates: [ugoe-motion, docs]
depends_on: [T0139]
assignee: claude-orchestrator
---

# T0158: ループするモーションが無限大や巨大な dt で止まらない

## 目的

`MotionPlayer::advance` (crates/ugoe-motion/src/player.rs) は、ループするモーションで `remaining -= duration - time` を 0 以下になるまで
繰り返す。T0139 のレビューで次を見つけた。

- `dt` が `+inf` だと `remaining` が減らず、終わらない (`Puppet::update(f32::INFINITY)` で止まった。`Puppet` は T0139 で負・非有限の `dt` を
  0 とみなすようにしたが、`ugoe-motion` を直接使うと残る)。
- 有限でも、`dt` の f32 の刻み (ulp) の半分より `duration` が短いと引き算で値が変わらず終わらない (例: `dt = 1e9`、`duration = 10`)。
  そうでなくても、`dt / duration` 回まわる。
- `dt` が NaN だと `dt <= 0.0` を通り抜け、再生の時刻が NaN になる (目パチ・呼吸 (`effects.rs`) も同じ)。

周回の数を割り算で求めて残りの時刻を `rem_euclid` で出すなど、回数が `dt` によらない形にする。1 フレームで複数周回したときのイベントの発火
(`docs/spec/motion.md#再生とフェード`: 周回ごとにすべて発火) の上限をどうするか (巨大な `dt` でイベントが膨れる) も決める。

## 受け入れ条件

- [ ] `MotionPlayer::advance` が `+inf`・NaN・巨大な `dt` で有限の時間で返り、時刻が有限のまま (決めた値) になることをテストで確かめた
- [ ] 目パチ・呼吸 (`EyeBlinkPlayer::advance`・`BreathPlayer::advance`) の NaN の `dt` を同じく扱った
- [ ] 1 フレームで複数周回したときのイベントの扱い (上限を設けるなら) を docs/spec/motion.md に書いた (公開仕様に関わるなら spec/motion.md も)
- [ ] `just ci` が通る

## 参考

- docs/spec/motion.md#再生とフェード
- crates/ugoe/src/lib.rs (`sanitize_dt`、T0139)
- T0317 (ugoe-motion のレビュー。重なる所見があればそちらと合わせる)

## メモ
- **閉じた (M9 のマージ):** T0317 (ugoe-motion のレビュー) で直した。ループするモーションは 1 回の advance の周回に上限を持ち、有限でない dt では何もしない。SDK の update も T0139 で不正な dt を 0 とみなす。
