import { readFileSync } from "node:fs";
import { DEFAULT_STORAGE_PREFIX, PlayerStorage, STORAGE_NAMES } from "@ugoe/player";
import { describe, expect, it } from "vitest";
import { STORAGE_KEYS, STORAGE_PREFIX } from "./storageKeys";

describe("localStorage keys", () => {
  const keys = Object.entries(STORAGE_KEYS);

  it("are all ugoe.viewer.<name> with the name of the entry", () => {
    expect(STORAGE_PREFIX).toBe("ugoe.viewer");
    for (const [name, key] of keys) {
      expect(name).toMatch(/^[a-z][A-Za-z0-9]*$/);
      expect(key).toBe(`${STORAGE_PREFIX}.${name}`);
    }
  });

  it("share the prefix with what the player remembers, without overlapping it", () => {
    // ビューアは storagePrefix を渡さないので、Player は既定の接頭辞で覚える。
    expect(DEFAULT_STORAGE_PREFIX).toBe(STORAGE_PREFIX);
    const player = new PlayerStorage();
    const playerKeys = Object.values(STORAGE_NAMES).map((name) => player.key(name));
    const all = [...keys.map(([, key]) => key), ...playerKeys];
    expect(new Set(all).size).toBe(all.length);
  });

  it("match the theme key read by index.html before the first paint", () => {
    const html = readFileSync(new URL("../../index.html", import.meta.url), "utf8");
    const read = [...html.matchAll(/localStorage\.getItem\("([^"]+)"\)/g)].map((m) => m[1]);
    expect(read).toEqual([STORAGE_KEYS.theme]);
  });
});
