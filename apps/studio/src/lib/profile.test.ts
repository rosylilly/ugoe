import { defaultLighting } from "@ugoe/player";
import { describe, expect, it } from "vitest";
import { describeError } from "./errors";
import { fileName, isStudioError, sceneLighting, withBackgroundKind } from "./profile";

describe("withBackgroundKind", () => {
  it("keeps the color of the same kind and uses defaults otherwise", () => {
    const color = { kind: "color", color: "#123456" } as const;
    expect(withBackgroundKind(color, "color")).toEqual(color);
    expect(withBackgroundKind(color, "chroma")).toEqual({ kind: "chroma", color: "#00ff00" });
    expect(withBackgroundKind({ kind: "transparent" }, "color")).toEqual({
      kind: "color",
      color: "#000000",
    });
    expect(withBackgroundKind(color, "transparent")).toEqual({ kind: "transparent" });
    // 画像はパスが無ければ変えない。
    expect(withBackgroundKind(color, "image")).toEqual(color);
    expect(withBackgroundKind(color, "image", "/bg.png")).toEqual({
      kind: "image",
      path: "/bg.png",
    });
  });
});

describe("helpers", () => {
  it("reads file names on any OS", () => {
    expect(fileName("/a/b/avatar.ugoe")).toBe("avatar.ugoe");
    expect(fileName("C:\\a\\avatar.ugoe")).toBe("avatar.ugoe");
  });

  it("uses the default lighting when the scene has none", () => {
    expect(sceneLighting({ lighting: null })).toEqual(defaultLighting());
    const broken = sceneLighting({ lighting: { enabled: false } as never });
    expect(broken.enabled).toBe(false);
    expect(broken.ambient).toEqual(defaultLighting().ambient);
  });

  it("recognizes backend errors and describes them", () => {
    const missing = { code: "missing", asset: "model", path: "/m/avatar.ugoe" };
    expect(isStudioError(missing)).toBe(true);
    expect(isStudioError(new Error("x"))).toBe(false);
    expect(isStudioError({ code: "nope" })).toBe(false);
    expect(describeError(missing)).toContain("/m/avatar.ugoe");
    expect(
      describeError({ code: "futureVersion", path: "p.json", version: 2, supported: 1 }),
    ).toMatch(/2.*1/);
    expect(describeError(new Error("boom"))).toBe("boom");
  });
});
