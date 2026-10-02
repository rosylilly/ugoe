import { describe, expect, it } from "vitest";
import { hexToRgb, parseHexColor, rgbToHex } from "./color";
import { acceptsFile, partitionFiles } from "./files";
import { mergeAttributes } from "./props";
import { ariaKeyShortcuts, formatShortcut, parseShortcut, shortcutKeys } from "./shortcut";

describe("shortcut", () => {
  it("reads Mod as Command on macOS and Ctrl elsewhere", () => {
    expect(parseShortcut("Mod+S", "mac")).toEqual({ modifiers: ["meta"], key: "S" });
    expect(parseShortcut("mod+s", "other")).toEqual({ modifiers: ["ctrl"], key: "S" });
  });

  it("orders the modifiers", () => {
    expect(shortcutKeys("Shift+Alt+Ctrl+Meta+K", "mac")).toEqual(["⌃", "⌥", "⇧", "⌘", "K"]);
    expect(shortcutKeys("Shift+Mod+K", "other")).toEqual(["Ctrl", "Shift", "K"]);
  });

  it("names special keys", () => {
    expect(formatShortcut("Mod+Enter", "mac")).toBe("⌘↩");
    expect(formatShortcut("Mod+Enter", "other")).toBe("Ctrl+Enter");
    expect(formatShortcut("Shift+ArrowUp", "other")).toBe("Shift+↑");
    expect(formatShortcut("Esc", "other")).toBe("Esc");
    expect(formatShortcut("Space", "mac")).toBe("Space");
    expect(formatShortcut("Mod++", "other")).toBe("Ctrl++");
  });

  it("writes aria-keyshortcuts", () => {
    expect(ariaKeyShortcuts("Mod+Shift+Z", "mac")).toBe("Shift+Meta+Z");
    expect(ariaKeyShortcuts("Mod+Shift+Z", "other")).toBe("Control+Shift+Z");
    expect(ariaKeyShortcuts("Shift+Space", "other")).toBe("Shift+Space");
  });
});

describe("color", () => {
  it("normalizes hex colors", () => {
    expect(parseHexColor("#FF8800")).toBe("#ff8800");
    expect(parseHexColor("0af")).toBe("#00aaff");
    expect(parseHexColor(" #abc ")).toBe("#aabbcc");
    expect(parseHexColor("#12345")).toBeNull();
    expect(parseHexColor("orange")).toBeNull();
  });

  it("converts between hex and RGB", () => {
    expect(hexToRgb("#ff8800")).toEqual({ r: 255, g: 136, b: 0 });
    expect(hexToRgb("nope")).toEqual({ r: 0, g: 0, b: 0 });
    expect(rgbToHex({ r: 255, g: 136, b: 0 })).toBe("#ff8800");
    expect(rgbToHex({ r: 300, g: -4, b: 12.6 })).toBe("#ff000d");
  });
});

describe("files", () => {
  const png = { name: "Face.PNG", type: "image/png" };
  const model = { name: "body.ugoe", type: "" };

  it("matches extensions and MIME types", () => {
    expect(acceptsFile(png, undefined)).toBe(true);
    expect(acceptsFile(png, ".png")).toBe(true);
    expect(acceptsFile(png, "image/*")).toBe(true);
    expect(acceptsFile(png, "image/jpeg")).toBe(false);
    expect(acceptsFile(model, ".ugoe, .psd")).toBe(true);
    expect(acceptsFile(model, "image/*")).toBe(false);
  });

  it("partitions files", () => {
    expect(partitionFiles([png, model], ".ugoe")).toEqual({ accepted: [model], rejected: [png] });
  });
});

describe("mergeAttributes", () => {
  it("chains handlers, keeps symbols and lets later values win", () => {
    const calls: string[] = [];
    const key = Symbol("attachment");
    const merged = mergeAttributes(
      { id: "a", onclick: () => calls.push("a"), [key]: 1 },
      { id: "b", onclick: () => calls.push("b"), title: undefined },
    );
    expect(merged.id).toBe("b");
    (merged.onclick as () => void)();
    expect(calls).toEqual(["a", "b"]);
    expect((merged as Record<symbol, unknown>)[key]).toBe(1);
    expect("title" in merged).toBe(true);
  });
});
