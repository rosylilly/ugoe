---
id: T1610
title: "エディタ: タイムラインの再生・シークを wasm の MotionSampler に切り替える"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1610
---

# T1610: エディタ: タイムラインの再生・シークを wasm の MotionSampler に切り替える

## 目的

T0451 で wasm に `MotionSampler` (カーブごとの `SegmentIndex` を持つ) を足したが、エディタのタイムライン
(`apps/editor/src/lib/panels/timeline/timeline.svelte.ts`) は毎回 `sample_motion_json(JSON.stringify(motion), t)` を呼ぶ。切り替えるまでエディタでの効果が無い (T0451 の候補)。

## 受け入れ条件

- [x] タイムラインが `MotionSampler` を持ち、選んだモーションが変わったとき・編集のたびに `set_motion_json` を呼ぶ。再生中は `sample(t)` だけを呼ぶ
- [x] 編集の後に古い索引で値を返さないことを確かめるテストがある (偽の wasm で)
- [x] `apps/editor/src/lib/viewport/ugoe-wasm-stub.d.ts` と docs/editor.md#タイムライン を更新した
- [x] `just ci` が通る

## 参考

- T0451 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0451 のエディタのタイムライン)。
- **やったこと:**
  - `TimelineState` (`timeline.svelte.ts`) が wasm の `MotionSampler` を 1 つ持ち、評価するモーションのオブジェクトが前と違うとき
    (選び直し・編集・Undo。ドキュメントは編集のたびに `motions` を `$state.raw` で新しい配列とオブジェクトに読み直す) だけ
    `set_motion_json` で渡し直す (最初は `new MotionSampler(json)`)。再生中のフレームとシークは `sample(t)` だけを呼ぶ。
    渡し直しが失敗したら (wasm は前のモーションを保つ) 値を返さず、次の評価でまた渡し直す。`dispose` で `free()` する。
  - `viewport/wasm.ts` から `MotionSampler` の型も出した。
  - テスト `timeline-state.test.ts` (偽の wasm): 同じモーションでは `sample` だけ・再生中は `sample` だけ・編集の後は渡し直して
    新しい値を返す (古い索引の値を返さない)・選び直しで渡し直す・失敗したら前の値を返さず次で渡し直す・`dispose` で解放。
  - docs/editor.md#タイムライン の「再生とシーク」を書き直した。
- **受け入れ条件の補足:** `apps/editor/src/lib/viewport/ugoe-wasm-stub.d.ts` は T0435 で消え、エディタの型は
  `bindings/web/stub/ugoe_wasm.d.ts` (T0451 で `MotionSampler` を追加済み) を使う。写しの変更は要らなかった (`just wasm-types` も通る)。
- **やらなかったこと:** 呼び出しの頻度を「モーションが変わって最初に評価するとき」にした (選び直し・編集の瞬間に先回りして渡すのではない)。
  評価しないあいだ (engaged でない編集など) は渡し直さないので、結果は同じで呼び出しが少ない。実際のエディタでの再生の手触りは見ていない。
- **起票の候補:** なし。
