// 巨大な dt や有限でない dt での時間の進み方 (spec/motion.md#host-supplied-numbers)。
// 仕様は 1 回の advance の段・周回の数に上限を許すだけで値を決めないので、時間の例では確かめられない。
// spec-check は参照実装と同じ上限を使い、例外を出さず、止まらないこと。

import assert from "node:assert/strict";
import { test } from "node:test";
import type { PartOverrides } from "../src/evaluate.ts";
import {
  Breath,
  EyeBlink,
  MAX_BLINK_PHASES_PER_ADVANCE,
  MAX_LAPS_PER_ADVANCE,
  PlayingMotion,
  PoseState,
} from "../src/motion.ts";
import type * as M from "../src/types.ts";

const blink = (over: Partial<M.EyeBlink> = {}): M.EyeBlink => ({
  parameters: ["ParamEyeLOpen"],
  interval: 4,
  closing: 0.1,
  closed: 0.05,
  opening: 0.15,
  seed: 1n,
  ...over,
});

const loopingMotion = (duration: number): M.Motion => ({
  name: "loop",
  duration,
  looping: true,
  fade_in: 0,
  fade_out: 0,
  curves: [],
  events: [{ t: 0.5 * duration, value: "half" }],
});

test("目パチは巨大な dt でも上限の段だけ進めて止まる", () => {
  for (const settings of [blink(), blink({ interval: 1e-30, closing: 0, closed: 0, opening: 0 })]) {
    const b = new EyeBlink(settings);
    b.advance(1e300);
    // 最初の間隔の後に 1 回 open に戻るたびに 1 つ引く (上限の段の数を 4 段ずつ回る)。
    assert.equal(b.intervals.length, 1 + MAX_BLINK_PHASES_PER_ADVANCE / 4);
    // 残りの時間は捨て、今の段の始めに留まる。
    assert.equal(b.phase, "open");
    assert.equal(b.t, 0);
    b.advance(1 / 60);
    assert.ok(Number.isFinite(b.t));
  }
});

test("目パチと呼吸は有限でない dt では進まない", () => {
  for (const dt of [Number.POSITIVE_INFINITY, Number.NaN, -1, 0]) {
    const b = new EyeBlink(blink());
    b.advance(dt);
    assert.equal(b.phase, "open");
    assert.equal(b.t, 0);
    assert.equal(b.intervals.length, 1);

    const breath = new Breath([
      { parameter: "ParamBreath", offset: 0, peak: 1, cycle: 3, weight: 1 },
    ]);
    breath.advance(dt);
    assert.deepEqual(breath.phase, [0]);
  }
});

test("ループするモーションは巨大な dt でも上限の周回だけ発火して止まる", () => {
  for (const duration of [1, 1e-30]) {
    const m = new PlayingMotion(loopingMotion(duration), "normal");
    const fired = m.advance(1e300);
    assert.ok(fired.length <= MAX_LAPS_PER_ADVANCE + 1, `${fired.length}`);
    assert.ok(m.time >= 0 && m.time < duration, `${m.time}`);
    m.advance(duration / 4);
    assert.ok(Number.isFinite(m.time));
  }
});

test("モーションは有限でない dt では進まない", () => {
  for (const dt of [Number.POSITIVE_INFINITY, Number.NaN]) {
    const m = new PlayingMotion(loopingMotion(1), "normal");
    assert.deepEqual(m.advance(dt), []);
    assert.equal(m.time, 0);
    assert.equal(m.elapsed, 0);
  }
});

test("ポーズは有限でない dt や負の dt ではフェードを進めない", () => {
  const pose: M.Pose = {
    fade_duration: 0.5,
    groups: [
      {
        entries: [
          { part: "A", links: [] },
          { part: "B", links: ["B2"] },
        ],
      },
    ],
  };
  for (const dt of [Number.POSITIVE_INFINITY, Number.NaN, -1, 0]) {
    const p = new PoseState(pose);
    // 半分までフェードしてから、有限でない dt のフレームを当てる。
    p.frame(new Map([["B", 1]]), 0.25, new Map());
    const overrides: PartOverrides = new Map();
    p.frame(new Map([["B", 1]]), dt, overrides);
    assert.deepEqual(p.opacity, [[0.5, 0.5]], `dt ${dt}`);
    for (const [part, value] of [
      ["A", 0.5],
      ["B", 0.5],
      ["B2", 0.5],
    ] as const) {
      assert.deepEqual(overrides.get(part), { value, weight: 1 }, `dt ${dt} ${part}`);
    }
    // 有限の dt が来れば続きからフェードする。
    p.frame(new Map(), 0.25, new Map());
    assert.deepEqual(p.opacity, [[0, 1]], `dt ${dt}`);
  }
});

test("ポーズは fade_duration が 0 なら dt によらずすぐに切り替える", () => {
  const pose: M.Pose = {
    fade_duration: 0,
    groups: [
      {
        entries: [
          { part: "A", links: [] },
          { part: "B", links: [] },
        ],
      },
    ],
  };
  for (const dt of [Number.POSITIVE_INFINITY, Number.NaN, -1, 0, 1 / 60]) {
    const p = new PoseState(pose);
    p.frame(new Map([["B", 1]]), dt, new Map());
    assert.deepEqual(p.opacity, [[0, 1]], `dt ${dt}`);
  }
});
