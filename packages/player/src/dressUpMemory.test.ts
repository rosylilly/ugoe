import { setLocale } from "@ugoe/i18n";
import { beforeEach, describe, expect, it } from "vitest";
import {
  type DressUpSet,
  describeRestoreFailure,
  dressUpMemory,
  loadAutoRestore,
  MAX_MODULES,
  needsRestore,
  parseDressUpSet,
  planRestore,
  saveAutoRestore,
} from "./dressUpMemory";
import type { ModelFile } from "./modelFile";

// 文言は日本語の訳で確かめる (英語の原文を確かめる it は、その中で言語を切り替える)。
beforeEach(() => setLocale("ja"));

class MemoryStorage implements Storage {
  #items = new Map<string, string>();
  get length() {
    return this.#items.size;
  }
  clear() {
    this.#items.clear();
  }
  getItem(key: string) {
    return this.#items.get(key) ?? null;
  }
  key(index: number) {
    return [...this.#items.keys()][index] ?? null;
  }
  removeItem(key: string) {
    this.#items.delete(key);
  }
  setItem(key: string, value: string) {
    this.#items.set(key, value);
  }
}

const set: DressUpSet = {
  modules: [
    {
      name: "LongHair",
      file: {
        name: "long-hair.ugoemod",
        fingerprint: "a",
        location: "sample:dress-up/long-hair.ugoemod",
      },
    },
    { name: "TwinTails", file: { name: "twin-tails.ugoemod", fingerprint: "b" } },
    {
      name: "Sailor",
      file: { name: "sailor.ugoemod", fingerprint: "c", location: "path:/m/sailor.ugoemod" },
    },
    { name: "CatEars", file: { name: "cat-ears.ugoemod", fingerprint: "d" } },
  ],
  attached: ["LongHair", "Sailor", "CatEars"],
};

describe("dress-up memory", () => {
  it("round-trips a set through storage", () => {
    const storage = new MemoryStorage();
    const memory = dressUpMemory(storage);
    const model = {
      name: "dress-up.ugoe",
      fingerprint: "m",
      location: "sample:dress-up/dress-up.ugoe",
    } as const;
    memory.set(model, set);
    expect(dressUpMemory(storage).get({ name: "", fingerprint: "m" })).toEqual(set);
  });

  it("drops malformed modules and attached names that were not loaded", () => {
    const parsed = parseDressUpSet({
      modules: [
        { name: "A", file: { name: "a", fingerprint: "1" } },
        { name: "", file: { fingerprint: "2" } },
        { name: "B" },
        { name: "A", file: { name: "a2", fingerprint: "3" } },
        "junk",
        { name: "C", file: { name: "c", fingerprint: "4", location: 7 } },
      ],
      attached: ["A", "Ghost", 3, "C", "A"],
    });
    expect(parsed).toEqual({
      modules: [
        { name: "A", file: { name: "a", fingerprint: "1" } },
        { name: "C", file: { name: "c", fingerprint: "4" } },
      ],
      attached: ["A", "C"],
    });
    expect(parseDressUpSet({ attached: [] })).toBeNull();
    expect(parseDressUpSet("x")).toBeNull();
    expect(parseDressUpSet({ modules: [], attached: "A" })).toEqual({ modules: [], attached: [] });
  });

  it("caps the number of modules per model", () => {
    const modules = Array.from({ length: MAX_MODULES + 10 }, (_, i) => ({
      name: `M${i}`,
      file: { name: `m${i}`, fingerprint: `${i}` },
    }));
    expect(parseDressUpSet({ modules, attached: [] })?.modules).toHaveLength(MAX_MODULES);
  });

  it("asks to restore when the attached set or order differs, or a module is missing", () => {
    const loaded = new Set(set.modules.map((m) => m.name));
    expect(needsRestore(set, loaded, set.attached)).toBe(false);
    expect(needsRestore(set, loaded, ["Sailor", "LongHair", "CatEars"])).toBe(true);
    expect(needsRestore(set, loaded, ["LongHair", "Sailor"])).toBe(true);
    expect(needsRestore(set, new Set(["LongHair", "Sailor", "CatEars"]), set.attached)).toBe(true);
  });

  it("finds each module loaded, in the session, at its location, or nowhere", () => {
    const cached: ModelFile = { name: "cat-ears.ugoemod", bytes: new Uint8Array() };
    const plan = planRestore(set, new Set(["LongHair"]), (fp) => (fp === "d" ? cached : undefined));
    expect(plan.map((p) => [p.module.name, p.source])).toEqual([
      ["LongHair", { kind: "loaded" }],
      ["TwinTails", { kind: "unavailable" }],
      ["Sailor", { kind: "location", location: "path:/m/sailor.ugoemod" }],
      ["CatEars", { kind: "cached", file: cached }],
    ]);
  });

  it("describes modules that cannot be restored", () => {
    const [, twin, sailor] = set.modules;
    if (twin === undefined || sailor === undefined) throw new Error("fixture");
    expect(describeRestoreFailure(twin)).toContain("「TwinTails」(twin-tails.ugoemod)");
    expect(describeRestoreFailure(twin)).toContain("もう一度開いてください");
    expect(describeRestoreFailure(sailor, "not found")).toContain("(not found)");
  });

  it("remembers the auto-restore choice and survives a broken storage", () => {
    const storage = new MemoryStorage();
    expect(loadAutoRestore(storage)).toBe(false);
    saveAutoRestore(true, storage);
    expect(loadAutoRestore(storage)).toBe(true);
    storage.getItem = () => {
      throw new Error("denied");
    };
    storage.setItem = () => {
      throw new Error("quota");
    };
    expect(loadAutoRestore(storage)).toBe(false);
    expect(() => saveAutoRestore(true, storage)).not.toThrow();
  });
});
