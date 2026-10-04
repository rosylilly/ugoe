import { describe, expect, it } from "vitest";
import { historyShortcut, isInDialog, isMac, isTextEntry, type KeyInput } from "./shortcuts";

const key = (k: string, mods: Partial<KeyInput> = {}): KeyInput => ({
  key: k,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  altKey: false,
  ...mods,
});

describe("history shortcuts", () => {
  it("uses Ctrl on Windows and Linux", () => {
    expect(historyShortcut(key("z", { ctrlKey: true }), false)).toBe("undo");
    expect(historyShortcut(key("Z", { ctrlKey: true, shiftKey: true }), false)).toBe("redo");
    expect(historyShortcut(key("y", { ctrlKey: true }), false)).toBe("redo");
    expect(historyShortcut(key("z", { metaKey: true }), false)).toBeNull();
  });

  it("uses Cmd on macOS", () => {
    expect(historyShortcut(key("z", { metaKey: true }), true)).toBe("undo");
    expect(historyShortcut(key("z", { metaKey: true, shiftKey: true }), true)).toBe("redo");
    expect(historyShortcut(key("y", { metaKey: true }), true)).toBeNull();
    expect(historyShortcut(key("z", { ctrlKey: true }), true)).toBeNull();
  });

  it("ignores other keys and modifiers", () => {
    expect(historyShortcut(key("z"), false)).toBeNull();
    expect(historyShortcut(key("a", { ctrlKey: true }), false)).toBeNull();
    expect(historyShortcut(key("z", { ctrlKey: true, altKey: true }), false)).toBeNull();
    expect(historyShortcut(key("y", { ctrlKey: true, shiftKey: true }), false)).toBeNull();
  });

  it("detects text entry targets", () => {
    const at = (match: boolean) =>
      ({ closest: () => (match ? {} : null) }) as unknown as EventTarget;
    expect(isTextEntry(at(true))).toBe(true);
    expect(isTextEntry(at(false))).toBe(false);
    expect(isTextEntry(null)).toBe(false);
  });

  it("detects targets inside dialogs", () => {
    const at = (selector: string | null) =>
      ({
        closest: (s: string) => (selector !== null && s.includes(selector) ? {} : null),
      }) as unknown as EventTarget;
    expect(isInDialog(at('[role="dialog"]'))).toBe(true);
    expect(isInDialog(at('[role="alertdialog"]'))).toBe(true);
    expect(isInDialog(at(null))).toBe(false);
    expect(isInDialog(null)).toBe(false);
  });

  it("detects macOS", () => {
    expect(isMac("MacIntel")).toBe(true);
    expect(isMac("Win32")).toBe(false);
    expect(isMac("Linux x86_64")).toBe(false);
  });
});
