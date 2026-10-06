import { readFileSync } from "node:fs";
import { PlayerStorage, STORAGE_NAMES } from "@ugoe/player";
import { describe, expect, it } from "vitest";
import { PLAYER_STORAGE_PREFIX, STORAGE_KEYS, STORAGE_PREFIX } from "./storageKeys";

// 名前は camelCase。`.` は分野の区切り (`dressUp.autoRestore`) にだけ使う (docs/ui.md#名前と切り替え)。
const NAME = /^[a-z][A-Za-z0-9]*$/;
const DOTTED_NAME = /^[a-z][A-Za-z0-9]*(\.[a-z][A-Za-z0-9]*)*$/;

describe("localStorage keys", () => {
  const keys = Object.entries(STORAGE_KEYS);

  it("are all ugoe.studio.<name> with the name of the entry", () => {
    expect(STORAGE_PREFIX).toBe("ugoe.studio");
    for (const [name, key] of keys) {
      expect(name).toMatch(NAME);
      expect(key).toBe(`${STORAGE_PREFIX}.${name}`);
    }
  });

  it("give the output window's player its own ugoe.studio.player prefix", () => {
    expect(PLAYER_STORAGE_PREFIX).toBe(`${STORAGE_PREFIX}.player`);
    const player = new PlayerStorage(PLAYER_STORAGE_PREFIX);
    const playerKeys = Object.values(STORAGE_NAMES).map((name) => player.key(name));
    const all = [...keys.map(([, key]) => key), ...playerKeys];
    expect(new Set(all).size).toBe(all.length);
  });

  it("name what the player remembers in camelCase, using dots only between areas", () => {
    for (const name of Object.values(STORAGE_NAMES)) {
      expect(name).toMatch(DOTTED_NAME);
    }
  });

  it("match the theme key read by index.html before the first paint", () => {
    const html = readFileSync(new URL("../../index.html", import.meta.url), "utf8");
    const read = [...html.matchAll(/localStorage\.getItem\("([^"]+)"\)/g)].map((m) => m[1]);
    expect(read).toEqual([STORAGE_KEYS.theme]);
  });
});
