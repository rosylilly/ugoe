import { describe, expect, it } from "vitest";
import {
  displayId,
  elementLabel,
  elementOptions,
  labelOf,
  nameOf,
  setHiddenIdPrefix,
  textureLabel,
} from "./labels";

describe("element labels", () => {
  it("shows the display name with the ID", () => {
    expect(elementLabel("顔", "Layer8")).toBe("顔 (Layer8)");
    expect(elementLabel("Face", "Face")).toBe("Face");
    expect(elementLabel("  ", "Warp")).toBe("Warp");
    expect(elementLabel(undefined, "Warp")).toBe("Warp");
  });

  it("looks up items by ID", () => {
    const items = [
      { id: "Layer8", name: "顔" },
      { id: "Warp", name: "Warp" },
    ];
    expect(labelOf(items, "Layer8")).toBe("顔 (Layer8)");
    expect(labelOf(items, "Nope")).toBe("Nope");
    expect(nameOf(items, "Layer8")).toBe("顔");
    expect(nameOf(items, "Nope")).toBe("Nope");
    expect(elementOptions(items, ["Warp"])).toEqual([{ id: "Layer8", label: "顔 (Layer8)" }]);
  });

  it("names textures after the meshes that use them", () => {
    expect(textureLabel("Layer8", [])).toBe("Layer8");
    expect(textureLabel("Layer8", ["顔"])).toBe("Layer8 — 顔");
    expect(textureLabel("atlas", ["a", "b", "c", "d"])).toBe("atlas — a, b +2");
    expect(textureLabel("Face", ["Face"])).toBe("Face");
  });
});

describe("hidden module prefix", () => {
  it("hides the prefix of a module project in labels only", () => {
    setHiddenIdPrefix("Hat/");
    try {
      expect(displayId("Hat/Brim")).toBe("Brim");
      expect(displayId("Hat/")).toBe("Hat/");
      expect(displayId("Face")).toBe("Face");
      expect(elementLabel("Brim", "Hat/Brim")).toBe("Brim");
      expect(elementLabel("つば", "Hat/Brim")).toBe("つば (Brim)");
      expect(nameOf([{ id: "Hat/Brim" }], "Hat/Brim")).toBe("Brim");
      expect(textureLabel("Hat/brim", ["Brim"])).toBe("brim — Brim");
    } finally {
      setHiddenIdPrefix(null);
    }
    expect(displayId("Hat/Brim")).toBe("Hat/Brim");
  });
});
