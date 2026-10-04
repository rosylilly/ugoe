import { defaultLighting } from "@ugoe/player";
import { describe, expect, it } from "vitest";
import { fileName, sceneLighting, withBackgroundKind, withExtension } from "./profile";

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

  it("adds the extension the save dialog left out", () => {
    expect(withExtension("/a/stream", "json")).toBe("/a/stream.json");
    expect(withExtension("/a/stream.JSON", "json")).toBe("/a/stream.JSON");
    expect(withExtension("/a/stream.ugoe", "json")).toBe("/a/stream.ugoe.json");
  });

  it("uses the default lighting when the scene has none", () => {
    expect(sceneLighting({ lighting: null })).toEqual(defaultLighting());
    const broken = sceneLighting({ lighting: { enabled: false } as never });
    expect(broken.enabled).toBe(false);
    expect(broken.ambient).toEqual(defaultLighting().ambient);
  });
});
