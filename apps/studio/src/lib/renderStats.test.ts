import { describe, expect, it } from "vitest";
import { RenderStatsWindow } from "./renderStats";

describe("render stats window", () => {
  it("summarizes about one second of draws", () => {
    const window = new RenderStatsWindow();
    expect(window.record({ at: 0, drawn: true, renderMs: 9, dropped: 0 })).toBeNull();
    let stats = null;
    // 20 ms ごとに 50 回 (1 秒)。1 回だけ 4 ms かかる。
    for (let i = 1; i <= 50 && stats === null; i++) {
      stats = window.record({ at: i * 20, drawn: true, renderMs: i === 7 ? 4 : 1, dropped: 2 });
    }
    expect(stats).not.toBeNull();
    expect(stats?.fps).toBeCloseTo(50, 5);
    expect(stats?.refreshFps).toBeCloseTo(50, 5);
    expect(stats?.renderMs).toBeCloseTo(53 / 50, 5);
    expect(stats?.renderMaxMs).toBe(4);
    expect(stats?.dropped).toBe(2);
    // 次のまとまりは新しく数える。
    expect(window.record({ at: 1020, drawn: true, renderMs: 1, dropped: 2 })).toBeNull();
  });

  it("counts only the draws but reports while nothing is drawn (T0857)", () => {
    const window = new RenderStatsWindow();
    window.record({ at: 0, drawn: false, renderMs: 0, dropped: 0 });
    let stats = null;
    // 144 Hz の画面 (約 6.9 ms ごと) で、刻み 30 のフレームが来た回 (5 回に 1 回) だけ描く。
    const period = 1000 / 144;
    for (let i = 1; stats === null; i++) {
      stats = window.record({ at: i * period, drawn: i % 5 === 0, renderMs: 2, dropped: 0 });
    }
    expect(stats.refreshFps).toBeCloseTo(144, 0);
    // 約 1 秒の中で描いた回数 (5 回に 1 回なので 28 か 29)。
    expect(Math.abs(stats.fps - 144 / 5)).toBeLessThan(1.5);
    expect(stats.renderMs).toBeCloseTo(2, 5);
    // 何も描かなかった 1 秒も報告する (「描いていない」にしない)。
    stats = null;
    for (let i = 1; stats === null; i++) {
      stats = window.record({ at: 1000 + i * period, drawn: false, renderMs: 0, dropped: 0 });
    }
    expect(stats.fps).toBe(0);
    expect(stats.refreshFps).toBeCloseTo(144, 0);
    expect(stats.renderMs).toBe(0);
    expect(stats.renderMaxMs).toBe(0);
  });
});
