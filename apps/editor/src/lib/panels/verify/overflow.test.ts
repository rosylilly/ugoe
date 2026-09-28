import { describe, expect, it } from "vitest";
import type { Model, OverflowFinding, OverflowReport } from "../../generated/types";
import {
  cellParameters,
  formatAmount,
  formatCell,
  formatCount,
  groupFindings,
  overflowDots,
  targetLabel,
  targetRef,
} from "./overflow";

const model = {
  parameters: [
    { id: "AngleX", name: "Angle X" },
    { id: "Smile", name: "Smile" },
  ],
  meshes: [{ id: "HairTip", name: "Hair tip" }],
  deformers: [{ id: "HairWarp", name: "" }],
} as unknown as Model;

function finding(id: string, amount: number, cell: Record<string, number>): OverflowFinding {
  return {
    target: { kind: "mesh", id },
    parent: "Face",
    kind: "vertex",
    cell,
    count: 2,
    worst_index: 0,
    max_amount: amount,
  };
}

describe("groupFindings", () => {
  it("groups by element and sorts by the largest amount", () => {
    const report: OverflowReport = {
      findings: [
        finding("A", 0.1, { AngleX: -30 }),
        finding("A", 0.3, { AngleX: 30 }),
        finding("B", 0.2, {}),
      ],
      elements: 2,
      cells: 3,
      truncated: [],
    };
    const groups = groupFindings(report);
    expect(groups.map((g) => [g.target.id, g.maxAmount])).toEqual([
      ["A", 0.3],
      ["B", 0.2],
    ]);
    expect(groups[0]?.findings.map((f) => f.max_amount)).toEqual([0.3, 0.1]);
    expect(groupFindings({ ...report, findings: [] })).toEqual([]);
  });
});

describe("formatting", () => {
  it("writes amounts as a percentage of the grid", () => {
    expect(formatAmount(0.124)).toBe("12%");
    expect(formatAmount(0.0123)).toBe("1.2%");
    expect(formatAmount(1.5)).toBe("150%");
  });

  it("writes cells with parameter names", () => {
    expect(formatCell({ AngleX: 30, Smile: 1 / 3 }, model)).toBe(
      "Angle X (AngleX) = 30, Smile = 0.333",
    );
    expect(formatCell({}, model)).toBe("(no axes)");
    expect(formatCell({ Unknown: 1 }, null)).toBe("Unknown = 1");
  });

  it("writes counts and element labels", () => {
    expect(formatCount("vertex", 1)).toBe("1 vertex");
    expect(formatCount("control_point", 3)).toBe("3 control points");
    expect(formatCount("origin", 1)).toBe("1 origin");
    expect(targetLabel({ kind: "mesh", id: "HairTip" }, model)).toBe("Hair tip (HairTip)");
    expect(targetLabel({ kind: "deformer", id: "HairWarp" }, model)).toBe("HairWarp");
  });
});

describe("navigation", () => {
  it("selects meshes and deformers and sets the cell's parameters", () => {
    expect(targetRef({ kind: "deformer", id: "W" })).toEqual({ kind: "deformer", id: "W" });
    expect(targetRef({ kind: "part", id: "P" })).toBeNull();
    expect(cellParameters({ AngleX: 30, Smile: 1 })).toEqual([
      ["AngleX", 30],
      ["Smile", 1],
    ]);
  });
});

describe("overflowDots", () => {
  it("places the overflowing points at their evaluated positions", () => {
    const dots = overflowDots(
      [
        {
          target: { kind: "mesh", id: "HairTip" },
          parent: "Face",
          kind: "vertex",
          points: [
            { index: 1, uv: [1.2, 0.5], amount: 0.2 },
            { index: 9, uv: [1.2, 0.5], amount: 0.2 },
          ],
        },
        {
          target: { kind: "deformer", id: "Head" },
          parent: "Face",
          kind: "origin",
          points: [{ index: 0, uv: [0.5, -0.05], amount: 0.05 }],
        },
        {
          target: { kind: "deformer", id: "Missing" },
          parent: "Face",
          kind: "control_point",
          points: [{ index: 0, uv: [2, 2], amount: 1.4 }],
        },
      ],
      (target) =>
        target.id === "HairTip" ? [0, 0, 10, 20] : target.id === "Head" ? [5, 6, 1, 0, 0, 1] : null,
      model,
    );
    // 位置の範囲外の番号と、位置を読めない要素は飛ばす。
    expect(dots).toEqual([
      { key: "mesh:HairTip#1", x: 10, y: 20, title: "Hair tip (HairTip) #1: 20% outside Face" },
      { key: "deformer:Head#0", x: 5, y: 6, title: "Head origin: 5.0% outside Face" },
    ]);
  });
});
