import { describe, expect, it } from "vitest";
import { describeLoadWarning, parseLoadWarnings, replaceWarnings } from "./loadWarnings";

describe("parseLoadWarnings", () => {
  it("reads load_warnings_json and skips malformed entries", () => {
    const list = parseLoadWarnings(
      JSON.stringify([
        { code: "normal_map_ktx2", warning: "n", texture: 2 },
        { code: "physics_removed", warning: "p", reason: "bad rig" },
        {
          code: "module_detached",
          warning: "m",
          module: "Dress",
          slot: "Outfit",
          error: { code: "unknown_slot", message: "e", slot: "Outfit" },
        },
        { code: "texture_ktx2" },
        { warning: "no code" },
      ]),
    );
    expect(list).toEqual([
      { code: "normal_map_ktx2", warning: "n", texture: 2 },
      { code: "physics_removed", warning: "p", reason: "bad rig" },
      {
        code: "module_detached",
        warning: "m",
        module: "Dress",
        slot: "Outfit",
        error: {
          code: "unknown_slot",
          message: "e",
          module: undefined,
          slot: "Outfit",
          parameter: undefined,
          mesh: undefined,
          attached: undefined,
        },
      },
    ]);
    expect(parseLoadWarnings("{}")).toEqual([]);
  });
});

describe("describeLoadWarning", () => {
  it("describes textures, normal maps and physics in Japanese", () => {
    expect(describeLoadWarning({ code: "texture_ktx2", warning: "", texture: 1 })).toBe(
      "テクスチャ 1 は KTX2 の画像で、まだ表示できないため白で代わりに描いています",
    );
    expect(describeLoadWarning({ code: "normal_map_ktx2", warning: "", texture: 0 })).toBe(
      "テクスチャ 0 の法線マップは KTX2 の画像で、まだ使えないため法線マップなしで描いています",
    );
    expect(describeLoadWarning({ code: "physics_removed", warning: "w", reason: "r" })).toBe(
      "物理の設定がこのモデルに合わないため外しました (r)",
    );
  });

  it("leaves unknown kinds and detached modules to the caller", () => {
    expect(describeLoadWarning({ code: "something_new", warning: "w" })).toBeNull();
    expect(describeLoadWarning({ code: "module_detached", warning: "w" })).toBeNull();
  });
});

describe("replaceWarnings", () => {
  it("keeps the order and the texts without structure", () => {
    const structured = [{ code: "texture_missing", warning: "b", texture: 4 }];
    expect(replaceWarnings(["a", "b"], structured, describeLoadWarning)).toEqual([
      "a",
      "テクスチャ 4 の画像がファイルに無いため、白で代わりに描いています",
    ]);
  });
});
