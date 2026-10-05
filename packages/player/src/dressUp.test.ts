import { setLocale } from "@ugoe/i18n";
import { beforeEach, describe, expect, it } from "vitest";
import {
  describeDressUpError,
  describeLoadWarnings,
  dressUpErrorInfo,
  groupBySlot,
  parseAvatarSlots,
  parseModules,
  parseSlots,
  visibleSlots,
} from "./dressUp";
import { describeLoadError } from "./loadError";
import { parseLoadWarnings } from "./loadWarnings";

// 文言は日本語の訳で確かめる (英語の原文を確かめる it は、その中で言語を切り替える)。
beforeEach(() => setLocale("ja"));

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
    warnings: ["texture 0: placeholder", "texture 1: odd"],
    warning_details: [{ code: "texture_ktx2", warning: "texture 0: placeholder", texture: 0 }],
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
    // 構造のある警告は日本語にし、構造の無い文はそのまま残す。
    expect(modules[1]?.warnings).toEqual([
      "テクスチャ 0 は読めない形式 (ETC2・ASTC・Basis など) の KTX2 の画像なので、白で代わりに描いています",
      "texture 1: odd",
    ]);
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
    expect(groups.some((g) => g.hidden)).toBe(false);
  });

  // T0715: アバタープロファイルがアプリに公開していないスロットは、利用者に見せない。
  it("hides slots the avatar profile does not publish", () => {
    const slots = [
      { id: "Hair", parameters: [] },
      { id: "Outfit", parameters: [] },
      { id: "Shoes", parameters: [] },
    ];
    const published = ["Hair", "Shoes"];
    expect(visibleSlots(slots, published).map((s) => s.id)).toEqual(["Hair", "Shoes"]);
    expect(visibleSlots(slots, null)).toEqual(slots);
    const groups = groupBySlot(slots, parseModules(modulesJson), published);
    // 公開していない Outfit は、そのスロットのモジュールを読み込んだときだけ `hidden` として後ろに出る。
    expect(groups.map((g) => [g.id, g.missing, g.hidden, g.modules.map((m) => m.name)])).toEqual([
      ["Hair", false, false, ["LongHair"]],
      ["Shoes", false, false, []],
      ["Outfit", false, true, ["Dress"]],
      ["Hat", true, false, ["Hat"]],
    ]);
    const withoutOutfit = groupBySlot(slots, parseModules(modulesJson).slice(0, 1), published);
    expect(withoutOutfit.map((g) => g.id)).toEqual(["Hair", "Shoes"]);
  });
});

describe("parseAvatarSlots", () => {
  it("reads the published slot ids, or null without a profile", () => {
    expect(parseAvatarSlots('["Hair","Outfit"]')).toEqual(["Hair", "Outfit"]);
    expect(parseAvatarSlots("[]")).toEqual([]);
    expect(parseAvatarSlots("null")).toBeNull();
    expect(parseAvatarSlots('{"slots":[]}')).toBeNull();
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
    // GPU を失った後の付け外し (T0457) は、モデルの読み込みと同じ文 (T0465)。
    expect(describeDressUpError(dressUpError("device_lost"))).toBe(
      describeLoadError(Object.assign(new Error("x"), { name: "LoadError", code: "device_lost" })),
    );
    expect(describeDressUpError(dressUpError("device_lost"))).toContain("GPU が止まりました");
    setLocale("en");
    expect(describeDressUpError(dressUpError("no_model"))).toBe(
      "Open a model (.ugoe or .json) first.",
    );
    setLocale("ja");
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
  const structured = parseLoadWarnings(
    JSON.stringify([
      { code: "texture_missing", warning: "texture 3: placeholder", texture: 3 },
      { code: "physics_removed", warning: "physics settings were removed: x", reason: "x" },
      {
        code: "module_detached",
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
        code: "module_detached",
        module: "Cape",
        slot: "Outfit",
        error: { code: "missing_parameter", message: "x", module: "Cape", parameter: "Tail" },
        warning: "Cape could not be put back",
      },
      { code: "something_new", warning: "a new kind of warning" },
    ]),
  );

  it("describes every warning from its code, in the order of the returned warnings", () => {
    // 文の警告は文面に関わらず `warning` と同じものを置き換える (照らし合わせない)。
    expect(
      describeLoadWarnings(
        [
          "texture 3: placeholder",
          "physics settings were removed: x",
          "module `Dress` was detached: module `Dress` targets unknown slot `Outfit`",
          "Cape could not be put back",
          "a new kind of warning",
          "no structure",
        ],
        structured,
      ),
    ).toEqual([
      "テクスチャ 3 の画像がファイルに無いため、白で代わりに描いています",
      "物理の設定がこのモデルに合わないため外しました (x)",
      "このモデルに合わないため、着せ替えの「Dress」を外しました (「Dress」を付けるスロット「Outfit」が、このモデルにありません)",
      "このモデルに合わないため、着せ替えの「Cape」を外しました (「Cape」に必要なパラメータ「Tail」が、このモデルにありません)",
      "a new kind of warning",
      "no structure",
    ]);
    expect(describeLoadWarnings(["physics settings were removed: x"], [])).toEqual([
      "physics settings were removed: x",
    ]);
  });
});
