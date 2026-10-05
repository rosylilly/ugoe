import { describe, expect, it } from "vitest";
import { describeLoadWarnings, describeModuleWarnings, parseLoadWarnings } from "./loadWarnings";

const json = JSON.stringify([
  { code: "normal_map_ktx2", warning: "n", texture: 0 },
  { code: "texture_ktx2", warning: "t", texture: 0 },
  { code: "texture_missing", warning: "m", texture: 2 },
  { code: "physics_removed", warning: "physics settings were removed: bad", reason: "bad" },
  { code: "module_detached", warning: "d", module: "Dress", slot: "Outfit", error: {} },
  { code: "unknown_optional_feature", warning: "f", feature: "test.optional" },
  { code: "unknown_value", warning: "v", path: "MODL/meshes/0/blend" },
  { code: "something_new", warning: "a new kind of warning" },
  { code: "texture_ktx2" },
]);

describe("parseLoadWarnings", () => {
  it("reads load_warnings_json and skips malformed entries", () => {
    const list = parseLoadWarnings(json);
    expect(list.map((w) => w.code)).toEqual([
      "normal_map_ktx2",
      "texture_ktx2",
      "texture_missing",
      "physics_removed",
      "module_detached",
      "unknown_optional_feature",
      "unknown_value",
      "something_new",
    ]);
    expect(list[2]).toEqual({ code: "texture_missing", warning: "m", texture: 2 });
    expect(list[3]?.reason).toBe("bad");
    expect(list[5]?.feature).toBe("test.optional");
    expect(list[6]?.path).toBe("MODL/meshes/0/blend");
    expect(parseLoadWarnings("{}")).toEqual([]);
  });
});

describe("describeLoadWarnings", () => {
  it("describes each code in English and leaves detached modules to the dress-up section", () => {
    expect(describeLoadWarnings(parseLoadWarnings(json), false)).toEqual([
      "The normal map of texture 0 is a KTX2 image in a format the preview can't read yet (such as ETC2, ASTC or Basis), so it is lit without it.",
      "Texture 0 is a KTX2 image in a format the preview can't read yet (such as ETC2, ASTC or Basis), so it is drawn white.",
      "Texture 2 has no image, so it is drawn white.",
      "The physics settings no longer fit this model and were turned off (bad).",
      'This file uses the feature "test.optional" from a newer version, so the preview shows a fallback for it.',
      "This file has a value this version doesn't know (MODL/meshes/0/blend), so the preview uses the default for it.",
      "a new kind of warning",
    ]);
  });

  it("drops the removed physics when the document's physics was passed again", () => {
    const list = parseLoadWarnings(json).filter((w) => w.code === "physics_removed");
    expect(describeLoadWarnings(list, true)).toEqual([]);
  });
});

describe("describeModuleWarnings", () => {
  it("words a module's warnings from warning_details and keeps the rest as they are", () => {
    const details = [
      { code: "normal_map_ktx2", warning: "n", texture: 1 },
      { code: "texture_missing", warning: "m", texture: 0 },
    ];
    expect(describeModuleWarnings(["m", "other", "n"], details)).toEqual([
      "Texture 0 has no image, so it is drawn white.",
      "other",
      "The normal map of texture 1 is a KTX2 image in a format the preview can't read yet (such as ETC2, ASTC or Basis), so it is lit without it.",
    ]);
    expect(describeModuleWarnings(["m"], undefined)).toEqual(["m"]);
  });
});
