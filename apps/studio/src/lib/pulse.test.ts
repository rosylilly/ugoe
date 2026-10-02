import { describe, expect, it } from "vitest";
import { Pulse } from "./pulse";

describe("Pulse", () => {
  it("calls the scheduled steps on each beat until they stop", () => {
    const pulse = new Pulse();
    pulse.connect();
    const calls: number[] = [];
    const stop = pulse.schedule((now) => calls.push(now));
    pulse.beat(1);
    pulse.beat(2);
    stop();
    pulse.beat(3);
    expect(calls).toEqual([1, 2]);
  });
});
