import { afterEach, describe, expect, it } from "vitest";
import { readStoredItem, removeStoredItem, writeStoredItem } from "./storage";
import { BrokenStorage, MemoryStorage, useLocalStorage } from "./testing";

afterEach(() => useLocalStorage(undefined));

describe("stored items", () => {
  it("reads, writes and removes an item", () => {
    const storage = new MemoryStorage();
    expect(readStoredItem("a", storage)).toBeNull();
    writeStoredItem("a", "1", storage);
    expect(readStoredItem("a", storage)).toBe("1");
    removeStoredItem("a", storage);
    expect(storage.keys()).toEqual([]);
  });

  it("uses globalThis.localStorage when no storage is given", () => {
    const storage = new MemoryStorage();
    useLocalStorage(storage);
    writeStoredItem("a", "1");
    expect(storage.getItem("a")).toBe("1");
    expect(readStoredItem("a")).toBe("1");
  });

  it("never throws when the storage cannot be used or is missing", () => {
    for (const storage of [new BrokenStorage(), undefined]) {
      useLocalStorage(storage);
      expect(readStoredItem("a")).toBeNull();
      expect(() => writeStoredItem("a", "1")).not.toThrow();
      expect(() => removeStoredItem("a")).not.toThrow();
    }
    // 引くだけで例外になる環境。
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      get() {
        throw new Error("SecurityError");
      },
    });
    expect(readStoredItem("a")).toBeNull();
    expect(() => writeStoredItem("a", "1")).not.toThrow();
    expect(() => removeStoredItem("a")).not.toThrow();
  });
});
