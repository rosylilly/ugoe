import { describe, expect, it } from "vitest";
import {
  describeDressUpError,
  describeLoadWarnings,
  dressUpErrorInfo,
  groupBySlot,
  parseDetachedModules,
  parseModules,
  parseSlots,
} from "./dressUp";

/** ugoe-wasm の `DressUpError` と同じ形の `Error`。 */
function dressUpError(code: string, fields: Record<string, string> = {}, text = "failed") {
  return Object.assign(new Error(text), { name: "DressUpError", code, ...fields });
}

const modulesJson = JSON.stringify([
  {
    name: "LongHair",
    slot: "Hair",
    requires: {},
    attached: true,
    textures: 1,
    warnings: [],
  },
  {
    name: "Dress",
    slot: "Outfit",
    requires: { parameters: ["BodySway"], avatar_level: 2 },
    attached: false,
    textures: 1,
    warnings: ["texture 0: placeholder"],
  },
  { name: "Hat", slot: "Hat", requires: {}, attached: false, textures: 0, warnings: [] },
  { slot: "Broken" },
]);

describe("parseModules", () => {
  it("reads modules_json and skips malformed entries", () => {
    const modules = parseModules(modulesJson);
    expect(modules.map((m) => m.name)).toEqual(["LongHair", "Dress", "Hat"]);
    expect(modules[0]?.attached).toBe(true);
    expect(modules[1]?.requires).toEqual({ parameters: ["BodySway"], avatar_level: 2 });
    expect(modules[1]?.warnings).toEqual(["texture 0: placeholder"]);
  });

  it("returns nothing for a non-array", () => {
    expect(parseModules("{}")).toEqual([]);
  });
});

describe("parseSlots", () => {
  it("keeps the id and the exposed parameters", () => {
    const slots = parseSlots(
      JSON.stringify([
        { id: "Hair", anchor: "Head", parameters: [] },
        { id: "Outfit", parameters: ["BodySway"] },
        { anchor: "x" },
      ]),
    );
    expect(slots).toEqual([
      { id: "Hair", parameters: [] },
      { id: "Outfit", parameters: ["BodySway"] },
    ]);
  });
});

describe("groupBySlot", () => {
  it("lists the model's slots in order and appends unknown slots", () => {
    const groups = groupBySlot(
      [
        { id: "Hair", parameters: [] },
        { id: "Outfit", parameters: [] },
        { id: "Shoes", parameters: [] },
      ],
      parseModules(modulesJson),
    );
    expect(groups.map((g) => [g.id, g.missing, g.modules.map((m) => m.name)])).toEqual([
      ["Hair", false, ["LongHair"]],
      ["Outfit", false, ["Dress"]],
      ["Shoes", false, []],
      ["Hat", true, ["Hat"]],
    ]);
    expect(groups[0]?.attached?.name).toBe("LongHair");
    expect(groups[1]?.attached).toBeNull();
  });
});

describe("describeDressUpError", () => {
  it("reads the code and the fields", () => {
    const info = dressUpErrorInfo(dressUpError("unknown_slot", { module: "Hat", slot: "Hat" }));
    expect(info).toMatchObject({ code: "unknown_slot", module: "Hat", slot: "Hat" });
  });

  it("explains the common failures in plain words", () => {
    expect(describeDressUpError(dressUpError("unknown_slot", { module: "Hat", slot: "Hat" }))).toBe(
      "「Hat」を付けるスロット「Hat」が、このモデルにありません",
    );
    expect(
      describeDressUpError(
        dressUpError("missing_parameter", { module: "Cape", parameter: "Tail" }),
      ),
    ).toBe("「Cape」に必要なパラメータ「Tail」が、このモデルにありません");
    expect(
      describeDressUpError(
        dressUpError("slot_occupied", { slot: "Hair", module: "B", attached: "A" }),
      ),
    ).toContain("「A」が付いている");
    expect(describeDressUpError(dressUpError("no_model"))).toContain("先にモデル");
  });

  it("adds the original message for detailed failures", () => {
    expect(describeDressUpError(dressUpError("compile", {}, "bad deformer"))).toBe(
      "合成したモデルを組み立てられません (bad deformer)",
    );
    expect(describeDressUpError(dressUpError("invalid_file", {}, "truncated"))).toContain(
      "(truncated)",
    );
  });

  it("falls back to the message for other errors", () => {
    expect(describeDressUpError(new Error("the GPU device was lost"))).toBe(
      "the GPU device was lost",
    );
    expect(describeDressUpError(dressUpError("something_new", {}, "new failure"))).toBe(
      "new failure",
    );
    expect(dressUpErrorInfo("plain").code).toBe("unknown");
  });
});

describe("describeLoadWarnings", () => {
  const detachedJson = JSON.stringify([
    {
      module: "Dress",
      slot: "Outfit",
      error: {
        code: "unknown_slot",
        message: "module `Dress` targets unknown slot `Outfit`",
        module: "Dress",
        slot: "Outfit",
      },
      warning: "module `Dress` was detached: module `Dress` targets unknown slot `Outfit`",
    },
    {
      module: "Cape",
      slot: "Outfit",
      error: { code: "missing_parameter", message: "x", module: "Cape", parameter: "Tail" },
      warning: "Cape could not be put back",
    },
    { slot: "Broken" },
  ]);

  it("reads detached_modules_json and skips malformed entries", () => {
    const detached = parseDetachedModules(detachedJson);
    expect(detached.map((d) => d.module)).toEqual(["Dress", "Cape"]);
    expect(detached[0]?.error).toMatchObject({ code: "unknown_slot", slot: "Outfit" });
    expect(detached[1]?.error.parameter).toBe("Tail");
    expect(parseDetachedModules("{}")).toEqual([]);
  });

  it("describes detached modules from their codes and keeps other warnings", () => {
    const detached = parseDetachedModules(detachedJson);
    // 文の警告は文面に関わらず `warning` と同じものを除く (照らし合わせない)。
    expect(
      describeLoadWarnings(
        [
          "texture 3: placeholder",
          "module `Dress` was detached: module `Dress` targets unknown slot `Outfit`",
          "Cape could not be put back",
        ],
        detached,
      ),
    ).toEqual([
      "texture 3: placeholder",
      "このモデルに合わないため、着せ替えの「Dress」を外しました (「Dress」を付けるスロット「Outfit」が、このモデルにありません)",
      "このモデルに合わないため、着せ替えの「Cape」を外しました (「Cape」に必要なパラメータ「Tail」が、このモデルにありません)",
    ]);
    expect(describeLoadWarnings(["physics settings were removed: x"], [])).toEqual([
      "physics settings were removed: x",
    ]);
  });
});
