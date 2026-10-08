import { describe, expect, it } from "vitest";
import { CHECKED, checkLimit, RETRIES, summarize } from "./check.mjs";

const result = (model, measurement, median, best = median) => ({
  model,
  measurement,
  best,
  median,
});

/** ページを 1 回開いた結果 (index.html の計測の並び)。 */
const run = (update, follow = update, noPhysics = 10) => [
  result("generated", "update", update),
  result("generated", "follow", follow),
  result("generated", "update without physics", noPhysics),
];

describe("summarize", () => {
  it("takes the median of the runs for each measurement", () => {
    const summary = summarize([run(70, 72), run(90, 60), run(80, 66)]);
    expect(summary.map((r) => [r.model, r.measurement, r.median])).toEqual([
      ["generated", "update", 80],
      ["generated", "follow", 66],
      ["generated", "update without physics", 10],
    ]);
  });
});

describe("checkLimit", () => {
  it("checks only update and follow, without measuring again when they are under the limit", async () => {
    let remeasured = 0;
    // 物理を止めた `update` は上限と比べない
    const rows = await checkLimit(summarize([run(70, 72, 900)]), 300, async () => {
      remeasured++;
      return run(1);
    });
    expect(remeasured).toBe(0);
    expect(rows.map((r) => r.measurement)).toEqual(CHECKED);
    expect(rows.every((r) => r.ok && r.limit === 300)).toBe(true);
  });

  it("passes when a run measured again is under the limit", async () => {
    const again = [run(400, 290), run(250, 500)];
    const rows = await checkLimit(summarize([run(350, 310)]), 300, async () => again.shift());
    // 1 回目の測り直しで update はまだ超え (350 → 350)、follow は下回る (310 → 290)。2 回目で update も下回る
    expect(again).toEqual([]);
    expect(rows.map((r) => [r.measurement, r.median, r.ok])).toEqual([
      ["update", 250, true],
      ["follow", 290, true],
    ]);
  });

  it("fails with the fastest median after measuring again RETRIES times", async () => {
    let remeasured = 0;
    const rows = await checkLimit(summarize([run(500, 100)]), 300, async () => {
      remeasured++;
      return run(450 - remeasured * 50, 200);
    });
    expect(remeasured).toBe(RETRIES);
    expect(rows.map((r) => [r.measurement, r.median, r.ok])).toEqual([
      ["update", 350, false],
      ["follow", 100, true],
    ]);
  });

  it("fails on NaN", async () => {
    const rows = await checkLimit(summarize([run(Number.NaN, 10)]), 300, async () =>
      run(Number.NaN, 10),
    );
    expect(rows[0].ok).toBe(false);
  });
});
