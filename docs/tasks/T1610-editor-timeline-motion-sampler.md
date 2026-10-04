---
id: T1610
title: "エディタ: タイムラインの再生・シークを wasm の MotionSampler に切り替える"
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee:
---

# T1610: エディタ: タイムラインの再生・シークを wasm の MotionSampler に切り替える

## 目的

T0451 で wasm に `MotionSampler` (カーブごとの `SegmentIndex` を持つ) を足したが、エディタのタイムライン
(`apps/editor/src/lib/panels/timeline/timeline.svelte.ts`) は毎回 `sample_motion_json(JSON.stringify(motion), t)` を呼ぶ。切り替えるまでエディタでの効果が無い (T0451 の候補)。

## 受け入れ条件

- [ ] タイムラインが `MotionSampler` を持ち、選んだモーションが変わったとき・編集のたびに `set_motion_json` を呼ぶ。再生中は `sample(t)` だけを呼ぶ
- [ ] 編集の後に古い索引で値を返さないことを確かめるテストがある (偽の wasm で)
- [ ] `apps/editor/src/lib/viewport/ugoe-wasm-stub.d.ts` と docs/editor.md#タイムライン を更新した
- [ ] `just ci` が通る

## 参考

- T0451 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0451 のエディタのタイムライン)。
