import { describe, expect, it } from "vitest";
import { formatValue, groupParameters, parseParameterList, sliderStep } from "./parameters";

const param = (id: string, group?: string) => ({
  id,
  name: id,
  min: -1,
  max: 1,
  default: 0,
  group,
});

describe("groupParameters", () => {
  it("follows the group order and keeps ungrouped parameters last", () => {
    const sections = groupParameters(
      {
        parameters: [param("A"), param("B", "Eye"), param("C", "Face"), param("D", "Eye")],
        groups: [
          { id: "Face", name: "Face" },
          { id: "Eye", name: "Eyes" },
          { id: "Mouth", name: "Mouth" },
        ],
      },
      "Other",
    );
    expect(sections.map((s) => [s.name, s.parameters.map((p) => p.id)])).toEqual([
      ["Face", ["C"]],
      ["Eyes", ["B", "D"]],
      ["Other", ["A"]],
    ]);
  });

  it("treats unknown groups as ungrouped", () => {
    const sections = groupParameters({ parameters: [param("A", "Nope")], groups: [] }, "Other");
    expect(sections).toEqual([{ id: null, name: "Other", parameters: [param("A", "Nope")] }]);
  });
});

describe("parseParameterList", () => {
  it("fills missing lists", () => {
    expect(parseParameterList("{}")).toEqual({ parameters: [], groups: [] });
  });
});

describe("sliderStep", () => {
  it("scales with the range", () => {
    expect(sliderStep({ min: -30, max: 30 })).toBeCloseTo(0.01);
    expect(sliderStep({ min: 0, max: 1 })).toBeCloseTo(0.001);
    expect(sliderStep({ min: 0, max: 0 })).toBe(0.001);
  });
});

describe("formatValue", () => {
  it("rounds to three decimals", () => {
    expect(formatValue(0.12345)).toBe("0.123");
    expect(formatValue(-0.0001)).toBe("0");
    expect(formatValue(30)).toBe("30");
  });
});
