import { describe, expect, it } from "vitest";
import { RenderStatsWindow } from "./renderStats";

describe("render stats window", () => {
  it("summarizes about one second of draws", () => {
    const window = new RenderStatsWindow();
    expect(window.record(0, 9, 0)).toBeNull();
    let stats = null;
    // 20 ms ごとに 50 回 (1 秒)。1 回だけ 4 ms かかる。
    for (let i = 1; i <= 50 && stats === null; i++) {
      stats = window.record(i * 20, i === 7 ? 4 : 1, 2);
    }
    expect(stats).not.toBeNull();
    expect(stats?.fps).toBeCloseTo(50, 5);
    expect(stats?.renderMs).toBeCloseTo(53 / 50, 5);
    expect(stats?.renderMaxMs).toBe(4);
    expect(stats?.dropped).toBe(2);
    // 次のまとまりは新しく数える。
    expect(window.record(1020, 1, 2)).toBeNull();
  });
});
