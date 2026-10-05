import { describe, expect, it } from "vitest";
import { defaultAction, displayKeys, hasModifier, isSupportedCode, keysFromPress } from "./hotkeys";

const press = (
  code: string,
  mods: Partial<Record<"ctrl" | "alt" | "shift" | "meta", boolean>> = {},
) => ({
  code,
  ctrlKey: mods.ctrl ?? false,
  altKey: mods.alt ?? false,
  shiftKey: mods.shift ?? false,
  metaKey: mods.meta ?? false,
});

describe("hotkey keys", () => {
  it("builds the plugin syntax from the physical key", () => {
    expect(keysFromPress(press("KeyA", { ctrl: true, shift: true }))).toBe("Ctrl+Shift+KeyA");
    expect(keysFromPress(press("F5", { alt: true }))).toBe("Alt+F5");
    expect(keysFromPress(press("Digit1", { meta: true, ctrl: true }))).toBe("Ctrl+Super+Digit1");
    expect(keysFromPress(press("Numpad7"))).toBe("Numpad7");
    // 修飾キーだけ・読めないキーは待つ。
    expect(keysFromPress(press("ShiftLeft", { shift: true }))).toBeNull();
    expect(keysFromPress(press("IntlYen", { ctrl: true }))).toBeNull();
    expect(isSupportedCode("F24")).toBe(true);
    expect(isSupportedCode("F25")).toBe(false);
  });

  it("shows the keys in the Kbd syntax", () => {
    expect(displayKeys("Ctrl+Shift+KeyA")).toBe("Ctrl+Shift+A");
    expect(displayKeys("Super+Digit3")).toBe("Meta+3");
    expect(displayKeys("Alt+ArrowUp")).toBe("Alt+ArrowUp");
    expect(hasModifier("Ctrl+KeyA")).toBe(true);
    expect(hasModifier("F1")).toBe(false);
  });

  it("fills the arguments of a new action with the first choice", () => {
    const options = {
      scenes: ["scene-1"],
      expressions: ["Smile"],
      motions: [],
      slots: ["Hair"],
      poses: ["ArmA", "ArmB"],
    };
    expect(defaultAction("scene", options)).toEqual({ kind: "scene", scene: "scene-1" });
    expect(defaultAction("expression", options)).toEqual({
      kind: "expression",
      expression: "Smile",
      mode: "set",
    });
    expect(defaultAction("motion", options)).toEqual({
      kind: "motion",
      motion: "",
      priority: "normal",
    });
    expect(defaultAction("dressUp", options)).toEqual({
      kind: "dressUp",
      slot: "Hair",
      module: null,
    });
    expect(defaultAction("pose", options)).toEqual({ kind: "pose", part: "ArmA" });
    expect(defaultAction("pose", { ...options, poses: [] })).toEqual({ kind: "pose", part: "" });
    expect(defaultAction("trackingPause", options)).toEqual({ kind: "trackingPause" });
  });
});
