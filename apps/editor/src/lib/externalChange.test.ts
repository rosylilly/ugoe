import { BrokenStorage, MemoryStorage, useLocalStorage } from "@ugoe/i18n/testing";
import { describe, expect, it } from "vitest";
import {
  baseAction,
  describeFiles,
  externalAction,
  isExternalChangeError,
  loadAutoReload,
  saveAutoReload,
} from "./externalChange";
import type { ExternalChange } from "./ipc";

const change: ExternalChange = { files: ["model.json"], missing: false, token: "abc" };
const clean = { modified: false, autoReload: false, dismissed: null };

describe("external change", () => {
  it("does nothing when nothing changed or the change was dismissed", () => {
    expect(externalAction(null, clean)).toBe("none");
    expect(externalAction(change, { ...clean, dismissed: "abc" })).toBe("none");
  });

  it("asks unless it may reload silently", () => {
    expect(externalAction(change, clean)).toBe("ask");
    expect(externalAction(change, { ...clean, dismissed: "other" })).toBe("ask");
    expect(externalAction(change, { ...clean, autoReload: true })).toBe("reload");
    // 未保存の変更があれば、自動で開き直す設定でも尋ねる
    expect(externalAction(change, { ...clean, modified: true, autoReload: true })).toBe("ask");
    // 消されたプロジェクトは開き直せない
    const missing = { files: [], missing: true, token: "missing" };
    expect(externalAction(missing, { ...clean, autoReload: true })).toBe("ask");
  });

  it("reloads the base automatically with the same setting, even with unsaved edits", () => {
    const base = { autoReload: false, dismissed: null };
    expect(baseAction(null, base)).toBe("none");
    expect(baseAction(change, base)).toBe("ask");
    expect(baseAction(change, { ...base, autoReload: true })).toBe("reload");
    expect(baseAction(change, { autoReload: true, dismissed: "abc" })).toBe("none");
    // 見つからない本体は選び直すので尋ねる
    const missing = { files: [], missing: true, token: "base-missing" };
    expect(baseAction(missing, { ...base, autoReload: true })).toBe("ask");
  });

  it("lists a few changed files", () => {
    expect(describeFiles(["model.json"])).toBe("model.json");
    expect(describeFiles(["a", "b", "c"])).toBe("a, b, c");
    expect(describeFiles(["a", "b", "c", "d"])).toBe("a, b, c and 1 more file");
    expect(describeFiles(["a", "b", "c", "d", "e"])).toBe("a, b, c and 2 more files");
  });

  it("recognizes the refused save", () => {
    expect(
      isExternalChangeError({
        code: "editor.external_change",
        args: {},
        message:
          "the project was changed outside the editor after it was opened; reload it, or save again to overwrite",
      }),
    ).toBe(true);
    expect(isExternalChangeError({ code: "editor.io", args: {}, message: "disk full" })).toBe(
      false,
    );
    expect(isExternalChangeError("disk full")).toBe(false);
  });
});

describe("remembered auto reload", () => {
  it("saves and loads the setting under the same key and value as before", () => {
    const storage = new MemoryStorage();
    useLocalStorage(storage);
    expect(loadAutoReload()).toBe(false);
    saveAutoReload(true);
    expect(storage.getItem("ugoe.editor.autoReload")).toBe("true");
    expect(loadAutoReload()).toBe(true);
    saveAutoReload(false);
    expect(storage.getItem("ugoe.editor.autoReload")).toBe("false");
    expect(loadAutoReload()).toBe(false);
    storage.setItem("ugoe.editor.autoReload", "yes");
    expect(loadAutoReload()).toBe(false);
  });

  it("is off when localStorage cannot be used", () => {
    for (const storage of [new BrokenStorage(), undefined]) {
      useLocalStorage(storage);
      expect(loadAutoReload()).toBe(false);
      expect(() => saveAutoReload(true)).not.toThrow();
    }
  });
});
