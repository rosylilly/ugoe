import { describe, expect, it } from "vitest";
import {
  formatValue,
  groupParameters,
  internalCount,
  parseParameterList,
  sliderStep,
  visibleSections,
} from "./parameters";

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

describe("visibleSections", () => {
  const sections = groupParameters(
    {
      parameters: [
        param("AngleX", "Face"),
        { ...param("PhysRelay", "Face"), internal: true },
        { ...param("PhysOnly"), internal: true },
      ],
      groups: [{ id: "Face", name: "Face" }],
    },
    "Other",
  );

  it("hides internal parameters unless asked", () => {
    const ids = (s: ReturnType<typeof visibleSections>) =>
      s.map((x) => [x.name, x.parameters.map((p) => p.id)]);
    expect(ids(visibleSections(sections, "", false))).toEqual([["Face", ["AngleX"]]]);
    expect(ids(visibleSections(sections, "", true))).toEqual([
      ["Face", ["AngleX", "PhysRelay"]],
      ["Other", ["PhysOnly"]],
    ]);
    expect(internalCount(sections)).toBe(2);
  });

  it("filters by id or name", () => {
    expect(visibleSections(sections, " relay ", true).map((s) => s.parameters.length)).toEqual([1]);
    expect(visibleSections(sections, "relay", false)).toEqual([]);
  });

  it("matches the group name and every word of the query (T0725)", () => {
    const many = groupParameters(
      {
        parameters: [
          { ...param("EyeLOpen", "Eye"), name: "Left eye open" },
          { ...param("EyeROpen", "Eye"), name: "Right eye open" },
          param("AngleX", "Face"),
        ],
        groups: [
          { id: "Eye", name: "Eyes" },
          { id: "Face", name: "Head" },
        ],
      },
      "Other",
    );
    const ids = (q: string) =>
      visibleSections(many, q, false).flatMap((s) => s.parameters.map((p) => p.id));
    expect(ids("head")).toEqual(["AngleX"]);
    expect(ids("eyes")).toEqual(["EyeLOpen", "EyeROpen"]);
    expect(ids("eye left")).toEqual(["EyeLOpen"]);
    expect(ids("  open   RIGHT ")).toEqual(["EyeROpen"]);
    expect(ids("head left")).toEqual([]);
  });
});
