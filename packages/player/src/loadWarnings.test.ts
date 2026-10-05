import { setLocale } from "@ugoe/i18n";
import { beforeEach, describe, expect, it } from "vitest";
import { describeLoadWarning, parseLoadWarnings, replaceWarnings } from "./loadWarnings";

// 文言は日本語の訳で確かめる (英語の原文を確かめる it は、その中で言語を切り替える)。
beforeEach(() => setLocale("ja"));

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
        { code: "unknown_optional_feature", warning: "f", feature: "test.optional" },
        { code: "unknown_value", warning: "v", path: "MODL/meshes/0/blend" },
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
      { code: "unknown_optional_feature", warning: "f", feature: "test.optional" },
      { code: "unknown_value", warning: "v", path: "MODL/meshes/0/blend" },
    ]);
    expect(parseLoadWarnings("{}")).toEqual([]);
  });
});

describe("describeLoadWarning", () => {
  it("describes textures, normal maps and physics in Japanese", () => {
    expect(describeLoadWarning({ code: "texture_ktx2", warning: "", texture: 1 })).toBe(
      "テクスチャ 1 は読めない形式 (ETC2・ASTC・Basis など) の KTX2 の画像なので、白で代わりに描いています",
    );
    expect(describeLoadWarning({ code: "normal_map_ktx2", warning: "", texture: 0 })).toBe(
      "テクスチャ 0 の法線マップは読めない形式 (ETC2・ASTC・Basis など) の KTX2 の画像なので、法線マップなしで描いています",
    );
    expect(describeLoadWarning({ code: "physics_removed", warning: "w", reason: "r" })).toBe(
      "物理の設定がこのモデルに合わないため外しました (r)",
    );
  });

  it("describes features and values from a newer version in Japanese", () => {
    expect(
      describeLoadWarning({
        code: "unknown_optional_feature",
        warning: "",
        feature: "test.optional",
      }),
    ).toBe(
      "このファイルは新しい版の機能「test.optional」を使っています。この版では代わりの方法で表示しています",
    );
    expect(
      describeLoadWarning({ code: "unknown_value", warning: "", path: "MOTN#0/curves/1" }),
    ).toBe(
      "このファイルには、この版が知らない値があります (MOTN#0/curves/1)。既定の扱いで表示しています",
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
