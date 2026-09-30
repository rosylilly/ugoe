import { describe, expect, it } from "vitest";
import type { ModuleStatus } from "./ipc";
import {
  baseElements,
  isBaseElement,
  isBaseNotFoundError,
  isValidModuleName,
  moduleLabel,
  moduleTitle,
  suggestModuleName,
} from "./moduleProject";

const status: ModuleStatus = {
  name: "Hat",
  base: "/home/me/avatar.ugoeproj",
  slot: {
    id: "Hair",
    anchor: "Head",
    part: null,
    draw_order_min: 550,
    draw_order_max: 650,
    replaces: [],
    parameters: [],
  },
  base_parts: ["Root"],
  base_deformers: ["Head"],
  base_meshes: ["Face"],
};

describe("baseElements", () => {
  it("marks the base elements by kind", () => {
    const base = baseElements(status);
    expect(isBaseElement(base, { kind: "mesh", id: "Face" })).toBe(true);
    expect(isBaseElement(base, { kind: "deformer", id: "Face" })).toBe(false);
    expect(isBaseElement(base, { kind: "mesh", id: "Hat/Brim" })).toBe(false);
    expect(baseElements(null).size).toBe(0);
  });
});

describe("labels and names", () => {
  it("describes the module", () => {
    expect(moduleLabel(status)).toBe("Module: Hair");
    expect(moduleTitle(status)).toBe(
      "Slot Hair of avatar.ugoeproj (read-only base: /home/me/avatar.ugoeproj)",
    );
    expect(moduleTitle({ ...status, base: "C:\\x\\a.ugoe" })).toContain("of a.ugoe");
  });
  it("suggests and checks module names", () => {
    expect(suggestModuleName("Hair")).toBe("Hair");
    expect(suggestModuleName("#a/b")).toBe("a_b");
    expect(isValidModuleName("Hat")).toBe(true);
    expect(isValidModuleName(" ")).toBe(false);
    expect(isValidModuleName("a/b")).toBe(false);
    expect(isValidModuleName("#a")).toBe(false);
  });
  it("recognizes a missing base", () => {
    expect(isBaseNotFoundError("base model not found: /x")).toBe(true);
    expect(isBaseNotFoundError(null)).toBe(false);
  });
});
