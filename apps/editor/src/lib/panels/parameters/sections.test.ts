// パラメータの一覧の区切りの開閉を覚える (docs/editor.md#パラメータのグループ。T1646)。
import { BrokenStorage, MemoryStorage, useLocalStorage } from "@ugoe/i18n/testing";
import { describe, expect, it } from "vitest";
import {
  loadClosedSections,
  MAX_CLOSED_SECTIONS,
  saveClosedSections,
  sectionKey,
  withSectionOpen,
} from "./sections";

describe("remembered parameter sections", () => {
  it("saves and loads the closed sections", () => {
    const storage = new MemoryStorage();
    useLocalStorage(storage);
    expect(loadClosedSections()).toEqual(new Set());
    let closed = withSectionOpen(new Set(), "Face", false);
    closed = withSectionOpen(closed, sectionKey(null), false);
    saveClosedSections(closed);
    expect(storage.getItem("ugoe.editor.parameterSectionsClosed")).toBe('["Face",""]');
    expect(loadClosedSections()).toEqual(new Set(["Face", ""]));
    saveClosedSections(withSectionOpen(closed, "Face", true));
    expect(loadClosedSections()).toEqual(new Set([""]));
  });

  it("opens everything when the stored value cannot be read", () => {
    const storage = new MemoryStorage();
    useLocalStorage(storage);
    for (const bad of ["{", '{"Face":true}', "3"]) {
      storage.setItem("ugoe.editor.parameterSectionsClosed", bad);
      expect(loadClosedSections()).toEqual(new Set());
    }
    // 文字列でないものは捨てる
    storage.setItem("ugoe.editor.parameterSectionsClosed", '["Face",1,null]');
    expect(loadClosedSections()).toEqual(new Set(["Face"]));
    for (const broken of [new BrokenStorage(), undefined]) {
      useLocalStorage(broken);
      expect(loadClosedSections()).toEqual(new Set());
      expect(() => saveClosedSections(new Set(["Face"]))).not.toThrow();
    }
  });

  it("keeps only the most recently closed sections", () => {
    const storage = new MemoryStorage();
    useLocalStorage(storage);
    let closed = new Set<string>();
    for (let i = 0; i < MAX_CLOSED_SECTIONS + 5; i++)
      closed = withSectionOpen(closed, `G${i}`, false);
    // 閉じ直したものは最後へ
    closed = withSectionOpen(closed, "G0", false);
    saveClosedSections(closed);
    const loaded = [...loadClosedSections()];
    expect(loaded).toHaveLength(MAX_CLOSED_SECTIONS);
    expect(loaded.at(-1)).toBe("G0");
    expect(loaded).not.toContain("G1");
  });
});
