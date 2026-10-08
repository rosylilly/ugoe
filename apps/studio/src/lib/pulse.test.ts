import { afterEach, describe, expect, it, vi } from "vitest";
import { Pulse } from "./pulse";

/** 処理にかかる時間を決められる時計。 */
function fakeClock() {
  const clock = { now: 0, read: () => clock.now };
  return clock;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Pulse", () => {
  it("calls the scheduled steps on each beat until they stop", () => {
    const clock = fakeClock();
    const pulse = new Pulse(clock.read);
    pulse.connect();
    const calls: number[] = [];
    const stop = pulse.schedule((now) => calls.push(now));
    for (const now of [1, 2]) {
      clock.now = now;
      pulse.beat(now);
    }
    stop();
    clock.now = 3;
    pulse.beat(3);
    expect(calls).toEqual([1, 2]);
  });

  it("skips beats while the previous work takes longer than the beat interval", () => {
    const clock = fakeClock();
    const pulse = new Pulse(clock.read);
    pulse.connect();
    const calls: number[] = [];
    // 1 回の処理に 200ms かかる (重い検出)。刻みは 16ms ごと。
    pulse.schedule((now) => {
      calls.push(now);
      clock.now = now + 200;
    });
    for (let now = 0; now <= 1000; now += 16) {
      clock.now = Math.max(clock.now, now);
      pulse.beat(clock.now);
    }
    // 処理が終わってから 200ms 空けるので、400ms ごと (0・400・800) にだけ動く。
    expect(calls).toEqual([0, 400, 800]);
  });

  it("does not skip beats when the work is light", () => {
    const clock = fakeClock();
    const pulse = new Pulse(clock.read);
    pulse.connect();
    let count = 0;
    pulse.schedule((now) => {
      count++;
      clock.now = now + 1;
    });
    for (let now = 0; now < 1000; now += 16) {
      clock.now = now;
      pulse.beat(now);
    }
    expect(count).toBe(63);
  });

  it("keeps the other steps and the next beats running when a step throws", () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    const clock = fakeClock();
    const pulse = new Pulse(clock.read);
    pulse.connect();
    const calls: number[] = [];
    pulse.schedule(() => {
      throw new Error("broken");
    });
    pulse.schedule((now) => calls.push(now));
    for (const now of [0, 16, 32]) {
      clock.now = now;
      pulse.beat(now);
    }
    expect(calls).toEqual([0, 16, 32]);
    expect(error).toHaveBeenCalledTimes(3);
  });

  it("tells the steps which pulse they are handling", () => {
    const clock = fakeClock();
    const pulse = new Pulse(clock.read);
    pulse.connect();
    const seen: (number | null)[] = [];
    pulse.schedule(() => seen.push(pulse.current));
    pulse.beat(0, 7);
    pulse.beat(16, "0");
    pulse.beat(32);
    expect(seen).toEqual([7, null, null]);
    // 処理の外では番号を持たない。
    expect(pulse.current).toBeNull();
  });
});
