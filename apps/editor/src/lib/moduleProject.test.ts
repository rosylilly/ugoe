import { describe, expect, it } from "vitest";
import type { ModuleProblem, ModuleStatus } from "./ipc";
import {
  baseElements,
  baseIsProject,
  canMoveRig,
  describeModuleProblem,
  isBaseElement,
  isBaseMesh,
  isBaseRig,
  isBaseTexture,
  isValidModuleName,
  MODULE_CANNOT,
  moduleLabel,
  moduleProblemFixes,
  moduleTitle,
  renamePrefix,
  requirementsOf,
  suggestModuleName,
  toggled,
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
  base_textures: ["face.png"],
  base_rigs: ["Sway"],
  base_colliders: [],
  // JSON では空の欄を書かない
  requires: { parameters: ["BodySway"] } as ModuleStatus["requires"],
  base_slots: [],
  base_parameters: ["AngleX", "BodySway"],
  base_avatar_level: null,
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
    const title = moduleTitle(status);
    expect(title.split("\n")[0]).toBe('Module "Hat" for slot Hair of avatar.ugoeproj');
    expect(title).toContain("Read-only base: /home/me/avatar.ugoeproj");
    // できないことも示す
    for (const line of MODULE_CANNOT) expect(title).toContain(line);
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
});

describe("base things", () => {
  it("tells base textures and rigs", () => {
    expect(isBaseTexture(status, "face.png")).toBe(true);
    expect(isBaseTexture(status, "Hat/brim.png")).toBe(false);
    expect(isBaseTexture(null, "face.png")).toBe(false);
    expect(isBaseRig(status, "Sway")).toBe(true);
    expect(isBaseRig(status, "Hat/Sway")).toBe(false);
  });
  it("fills the omitted requirements", () => {
    expect(requirementsOf(status)).toEqual({
      avatar_level: null,
      slots: [],
      parameters: ["BodySway"],
    });
  });
  it("toggles in the given order", () => {
    const order = ["A", "B", "C"];
    expect(toggled(["C"], "A", true, order)).toEqual(["A", "C"]);
    expect(toggled(["A", "C"], "A", false, order)).toEqual(["C"]);
  });
});

describe("renamePrefix", () => {
  it("moves the selection to the new prefix", () => {
    expect(
      renamePrefix(
        [
          { kind: "mesh", id: "Hat/Brim" },
          { kind: "mesh", id: "Face" },
          { kind: "parameter", id: "Hat/X" },
        ],
        "Hat",
        "Cap",
      ),
    ).toEqual([
      { kind: "mesh", id: "Cap/Brim" },
      { kind: "mesh", id: "Face" },
      { kind: "parameter", id: "Hat/X" },
    ]);
  });
});

describe("module problems", () => {
  const problem = (kind: ModuleProblem["kind"], base = "/p/avatar.ugoeproj"): ModuleProblem => ({
    kind,
    base,
    slot: "Hair",
    base_slots: kind === "unknown_slot" ? ["Outfit"] : [],
    message: "x",
  });
  it("offers fixes for each kind", () => {
    expect(moduleProblemFixes(problem("base_not_found"))).toEqual([
      "locate_project",
      "locate_bundle",
    ]);
    expect(moduleProblemFixes(problem("unknown_slot"))).toEqual([
      "choose_slot",
      "open_base",
      "locate_project",
      "locate_bundle",
    ]);
    // 書き出した本体 (.ugoe) はエディタで開いて直せない
    expect(moduleProblemFixes(problem("does_not_fit", "/p/avatar.ugoe"))).toEqual([
      "locate_project",
      "locate_bundle",
    ]);
    expect(baseIsProject("/p/a.UGOE")).toBe(false);
  });
  it("describes what happened", () => {
    expect(describeModuleProblem(problem("base_not_found"))).toContain("moved or renamed");
    expect(describeModuleProblem(problem("unknown_slot"))).toContain('no longer has slot "Hair"');
    expect(describeModuleProblem(problem("does_not_fit"))).toContain("no longer fits");
    // ファイル名は引用符で囲み、文と続けて読めないようにする。
    expect(describeModuleProblem(problem("unknown_slot", "/p/moved.ugoeproj"))).toMatch(
      /^The base model "moved\.ugoeproj" no longer has slot "Hair"\./,
    );
  });
});

describe("rig order in a module project", () => {
  const module = { base_rigs: ["BaseA", "BaseB"] } as unknown as ModuleStatus;
  const rigs = ["BaseA", "BaseB", "Hat/Sway", "Hat/Ribbon"];
  it("reorders only the module's rigs among themselves", () => {
    expect(canMoveRig(module, rigs, 0, 1)).toBe(false);
    expect(canMoveRig(module, rigs, 2, -1)).toBe(false);
    expect(canMoveRig(module, rigs, 2, 1)).toBe(true);
    expect(canMoveRig(module, rigs, 3, -1)).toBe(true);
    expect(canMoveRig(module, rigs, 3, 1)).toBe(false);
    // 本体のモデルのプロジェクトでは、端でなければ動かせる。
    expect(canMoveRig(null, rigs, 0, 1)).toBe(true);
    expect(canMoveRig(null, rigs, 0, -1)).toBe(false);
  });
  it("knows the base meshes", () => {
    const m = { base_meshes: ["Face"] } as unknown as ModuleStatus;
    expect(isBaseMesh(m, "Face")).toBe(true);
    expect(isBaseMesh(m, "Hat/Brim")).toBe(false);
    expect(isBaseMesh(null, "Face")).toBe(false);
  });
});
