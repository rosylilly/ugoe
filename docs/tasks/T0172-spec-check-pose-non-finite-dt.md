---
id: T0172
title: spec-check のポーズが有限でない dt でパーツの不透明度を NaN にしないようにする
status: done
milestone: M9
crates: [tools/spec-check, spec, docs]
depends_on: []
assignee: claude-worker-T0172
---

# T0172: spec-check のポーズが有限でない dt でパーツの不透明度を NaN にしないようにする

## 目的

T0164 で spec-check (仕様だけから書いた別の実装) のモーション・表情・目パチ・呼吸が無限大の `dt` で進んでいたのを直した。
ポーズ (`tools/spec-check/src/motion.ts` の `PoseState.frame`) は `Math.max(dt, 0) / fade` で不透明度を進めるので、`dt` が NaN なら不透明度が NaN になり、
無限大なら一度で切り替わる (T0164 の候補)。公開仕様 `spec/motion.md#pose` と `#host-supplied-numbers` に照らして扱いを確かめ、
仕様が決めていなければ先に仕様に書く (spec-check の独立性の規則)。

## 受け入れ条件

- [x] `spec/motion.md#pose` (と `#host-supplied-numbers`) が有限でない `dt` のポーズの扱いを決めている。決めていなければ足し、`docs/spec/motion.md` の対応する節も直した (参照実装 `ugoe-motion` の挙動と食い違わないことを確かめた)
- [x] spec-check のポーズが仕様どおりに扱い、NaN・無限大・負の `dt` のテストが `tools/spec-check/test/` にある
- [x] `just spec` が通る
- [x] `just ci` が通る

## 参考

- T0164 のメモ
- spec/motion.md#pose、docs/spec/public-spec.md

## メモ

- T0032 で起票した (元の候補: T0164 (ポーズの dt))。
- 仕様: `spec/motion.md#host-supplied-numbers` はフレームの単位で「有限でない・負の `dt` は `dt = 0` として進める」と決めていたが、
  `#pose` の手順 2 は `max(dt, 0) / fade_duration` と書かれていて、段だけを読むと NaN の `dt` で NaN、無限大で一度に切り替わると読めた。
  `#pose` の手順 2 に「有限の `dt > 0` だけがフェードを進める (`fade_duration ≤ 0` なら `dt` によらずすぐ 1。ほかの要素を抑えるのは進めないときも行う)」
  と書き、`#host-supplied-numbers` の「有限の `dt > 0` だけで進むもの」の列にポーズのフェードを足した。`docs/spec/motion.md#ポーズ` も同じに直した。
- spec-check: `PoseState.frame` を `advances(dt)` (有限の `dt > 0`) で進めるようにした。テストは `tools/spec-check/test/motion.test.ts`
  (無限大・NaN・負・0 の `dt` でフェードが止まり上書きは書かれ、有限の `dt` で続きから進む。`fade_duration` 0 は `dt` によらずすぐ切り替わる)。
  直す前のコードで無限大の `dt` の試験が落ちることを確かめた。
- 参照実装: `Puppet::update` は `sanitize_dt` で元から 0 にしていたので、フレームの挙動は仕様と同じ。ただ `PosePlayer::update`/`update_by` を
  直接使うと無限大の `dt` で一度に切り替わり、`docs/spec/motion.md#ホストから来る数` の「再生器を直接使うときは、正の有限の `dt` でなければ何も進めない」
  と食い違っていた (NaN は Rust の `f32::max` が 0 にしていた)。範囲外 (`crates/ugoe-motion`) だが小さいので直し、テスト
  `pose::tests::non_finite_or_negative_dt_does_not_advance_the_fade` を足した (直す前に落ちることを確かめた)。呼び出し元は `Puppet` だけで、挙動は変わらない。
- `just spec`・`just ci`・`just wasm-check` が通った。
- 起票の候補: なし。
