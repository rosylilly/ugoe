---
id: T0451
title: "wasm: カーブの値を返す API でもセグメントの索引 (SegmentIndex) を使う"
status: done
milestone: M9
crates: [ugoe-wasm]
depends_on: []
assignee: claude-worker-T0451
---

# T0451: wasm: カーブの値を返す API でもセグメントの索引 (SegmentIndex) を使う

## 目的

T0324 でモーションの再生はカーブのセグメントを索引 (`SegmentIndex`) で引くようになったが、`ugoe-wasm` の `animation.rs`
(フェードなどを掛けないカーブの値そのものを返す API。エディタのタイムラインのプレビューが使う) は、毎回線形の `curve::evaluate` を使う。
毎フレーム呼ぶ使い方なので、カーブごとに索引を持たせる。

## 受け入れ条件

- [x] `animation.rs` のカーブの値の評価が索引を使い、結果が `curve::evaluate` と一致するテスト (ネイティブのテスト) がある
- [x] モーションを差し替えたときに索引を作り直す
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0324 のメモ (起票の候補)
- crates/ugoe-wasm/src/animation.rs、crates/ugoe-motion/src/curve.rs

## メモ

- T0019 で起票した (元の候補: T0324)。
- **やったこと:**
  - `animation.rs` に `MotionSampler` を足した。未知の対象のカーブを除いたモーションと、カーブごとの `SegmentIndex` を持ち、
    `sample_json(t)` で `sample_motion_json` と同じ形の JSON を返す。`set_motion_json` でモーションを差し替えると索引を作り直す
    (読めない JSON ではエラーにし、今のモーションを保つ)。
  - `sample_motion_json(json, t)` は `MotionSampler::from_json(json)?.sample_json(t)` にした (形・結果は変わらない)。
  - wasm に `MotionSampler` クラス (`new MotionSampler(json)`・`set_motion_json(json)`・`sample(t)`) を出した。
  - テスト: `motion_sampler_matches_linear_evaluation` (線形・ベジェ・ステップ・逆ステップ・2 つ目の `start`・未知のセグメント・
    未知の対象を含むモーションを、-1〜5 秒と区間の境目・NaN・無限大で `curve::evaluate` と比べる)、
    `motion_sampler_rebuilds_indices_when_the_motion_is_replaced`。
- **`crates:` の外の小さな変更:** `bindings/web/stub/ugoe_wasm.d.ts` に `MotionSampler` の型を足した
  (bindings/web/README.md の「ugoe-wasm の公開 API を変えたらスタブを作り直す」に従う)。`bindings/web/src/index.ts` からは出していない。
- **やらなかったこと:** エディタのタイムライン (`apps/editor/src/lib/panels/timeline/timeline.svelte.ts`) は、まだ毎回
  `sample_motion_json(JSON.stringify(motion), t)` を呼ぶ (`crates:` の外)。`MotionSampler` に切り替えるまで、エディタでの効果は無い。
- **起票の候補:**
  - エディタ: タイムラインの再生・シークを `MotionSampler` に切り替える (選んだモーションが変わったとき・編集のたびに
    `set_motion_json` を呼ぶ)。`apps/editor/src/lib/viewport/ugoe-wasm-stub.d.ts` と `docs/editor.md#タイムライン` も直す。
  - @ugoe/runtime: `MotionSampler` を `bindings/web/src/index.ts` から出すか決める (今は `sample_motion_json` だけを出している)。
