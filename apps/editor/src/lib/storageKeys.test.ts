import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { LEFT_WIDTH, RIGHT_SPLIT, RIGHT_WIDTH, TIMELINE_HEIGHT } from "./layout";
import { STORAGE_KEYS, STORAGE_PREFIX } from "./storageKeys";

describe("localStorage keys", () => {
  const keys = Object.entries(STORAGE_KEYS);

  it("are all ugoe.editor.<name> with the name of the entry", () => {
    expect(STORAGE_PREFIX).toBe("ugoe.editor");
    for (const [name, key] of keys) {
      expect(name).toMatch(/^[a-z][A-Za-z0-9]*$/);
      expect(key).toBe(`${STORAGE_PREFIX}.${name}`);
    }
  });

  it("are not shared by two uses", () => {
    expect(new Set(keys.map(([, key]) => key)).size).toBe(keys.length);
  });

  it("are used by the layout sizes", () => {
    expect([LEFT_WIDTH, RIGHT_WIDTH, TIMELINE_HEIGHT, RIGHT_SPLIT].map((r) => r.key)).toEqual([
      STORAGE_KEYS.leftWidth,
      STORAGE_KEYS.rightWidth,
      STORAGE_KEYS.timelineHeight,
      STORAGE_KEYS.rightSplit,
    ]);
  });

  it("match the theme key read by index.html before the first paint", () => {
    const html = readFileSync(new URL("../../index.html", import.meta.url), "utf8");
    const read = [...html.matchAll(/localStorage\.getItem\("([^"]+)"\)/g)].map((m) => m[1]);
    expect(read).toEqual([STORAGE_KEYS.theme]);
  });
});
