import { describe, expect, it } from "vitest";
import { DriverSet } from "./drivers";

describe("DriverSet", () => {
  it("lets later drivers win and skips non-finite values", () => {
    const set = new DriverSet();
    set.add({
      name: "a",
      frame: () => [
        ["X", 1],
        ["Y", 2],
      ],
    });
    set.add({
      name: "b",
      frame: () =>
        new Map([
          ["Y", 3],
          ["Z", Number.NaN],
        ]),
    });
    expect([...set.collect(1 / 60)]).toEqual([
      ["X", 1],
      ["Y", 3],
    ]);
  });

  it("passes dt and can remove drivers", () => {
    const set = new DriverSet();
    const seen: number[] = [];
    const remove = set.add({
      name: "t",
      frame: (dt) => {
        seen.push(dt);
        return null;
      },
    });
    set.collect(0.5);
    remove();
    set.collect(0.25);
    expect(seen).toEqual([0.5]);
    expect(set.size).toBe(0);
  });
});
