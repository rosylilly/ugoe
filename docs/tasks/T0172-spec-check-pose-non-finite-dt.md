---
id: T0172
title: spec-check のポーズが有限でない dt でパーツの不透明度を NaN にしないようにする
status: in-progress
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

- [ ] `spec/motion.md#pose` (と `#host-supplied-numbers`) が有限でない `dt` のポーズの扱いを決めている。決めていなければ足し、`docs/spec/motion.md` の対応する節も直した (参照実装 `ugoe-motion` の挙動と食い違わないことを確かめた)
- [ ] spec-check のポーズが仕様どおりに扱い、NaN・無限大・負の `dt` のテストが `tools/spec-check/test/` にある
- [ ] `just spec` が通る
- [ ] `just ci` が通る

## 参考

- T0164 のメモ
- spec/motion.md#pose、docs/spec/public-spec.md

## メモ

- T0032 で起票した (元の候補: T0164 (ポーズの dt))。
