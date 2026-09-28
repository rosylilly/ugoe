import { describe, expect, it } from "vitest";
import { pressPoint, trianglesPath, validIndices } from "./form";

describe("pressPoint", () => {
  it("selects only the pressed point when it is not selected", () => {
    expect(pressPoint([1, 2], 5, false)).toEqual({ picked: [5], drag: true });
  });

  it("keeps the selection when a selected point is pressed (drag moves all)", () => {
    expect(pressPoint([1, 2, 5], 2, false)).toEqual({ picked: [1, 2, 5], drag: true });
  });

  it("toggles with Shift and does not drag a point it removed", () => {
    expect(pressPoint([1, 2], 5, true)).toEqual({ picked: [1, 2, 5], drag: true });
    expect(pressPoint([1, 2, 5], 2, true)).toEqual({ picked: [1, 5], drag: false });
  });
});

describe("validIndices", () => {
  it("drops indices outside the point count", () => {
    expect(validIndices([0, 3, 4, -1, 1.5], 4)).toEqual([0, 3]);
  });
});

describe("trianglesPath", () => {
  it("draws each triangle and skips ones with missing points", () => {
    const points: [number, number][] = [
      [0, 0],
      [10, 0],
      [0, 10],
    ];
    expect(trianglesPath(points, [0, 1, 2, 0, 2, 7])).toBe("M0,0L10,0L0,10Z");
  });
});
