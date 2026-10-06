import { BrokenStorage, MemoryStorage } from "@ugoe/i18n/testing";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_STORAGE_PREFIX, PlayerStorage, STORAGE_NAMES } from "./storage";

afterEach(() => vi.unstubAllGlobals());

describe("PlayerStorage", () => {
  it("uses the viewer's prefix when the prefix is omitted", () => {
    const storage = new MemoryStorage();
    const store = new PlayerStorage(undefined, storage);
    expect(store.prefix).toBe(DEFAULT_STORAGE_PREFIX);
    expect(Object.values(STORAGE_NAMES).map((name) => store.key(name))).toEqual([
      "ugoe.viewer.lighting",
      "ugoe.viewer.lighting.models",
      "ugoe.viewer.lighting.markers",
      "ugoe.viewer.dress-up",
      "ugoe.viewer.dress-up.auto-restore",
      "ugoe.viewer.idle-motion.models",
      "ugoe.viewer.auto-idle",
      "ugoe.viewer.motion-over-tracking",
      "ugoe.viewer.motion-over-tracking.kept",
      "ugoe.viewer.remembered-order",
    ]);
  });

  it("reads and writes only under its own prefix", () => {
    const storage = new MemoryStorage();
    const viewer = new PlayerStorage(undefined, storage);
    const studio = new PlayerStorage("ugoe.studio.player", storage);
    viewer.set(STORAGE_NAMES.rememberedOrder, "name");
    expect(studio.get(STORAGE_NAMES.rememberedOrder)).toBeNull();
    studio.setJson(STORAGE_NAMES.trackingKept, ["EyeLOpen"]);
    expect(storage.getItem("ugoe.studio.player.motion-over-tracking.kept")).toBe('["EyeLOpen"]');
    expect(viewer.getJson(STORAGE_NAMES.trackingKept)).toBeUndefined();
    expect(studio.getJson(STORAGE_NAMES.trackingKept)).toEqual(["EyeLOpen"]);
  });

  it("remembers on and off and falls back otherwise", () => {
    const store = new PlayerStorage("p", new MemoryStorage());
    expect(store.getToggle(STORAGE_NAMES.autoIdle, true)).toBe(true);
    store.setToggle(STORAGE_NAMES.autoIdle, false);
    expect(store.getToggle(STORAGE_NAMES.autoIdle, true)).toBe(false);
    store.setToggle(STORAGE_NAMES.autoIdle, true);
    expect(store.getToggle(STORAGE_NAMES.autoIdle, false)).toBe(true);
    store.set(STORAGE_NAMES.autoIdle, "maybe");
    expect(store.getToggle(STORAGE_NAMES.autoIdle, false)).toBe(false);
  });

  it("treats broken JSON as nothing remembered", () => {
    const store = new PlayerStorage("p", new MemoryStorage());
    store.set(STORAGE_NAMES.lighting, "{not json");
    expect(store.getJson(STORAGE_NAMES.lighting)).toBeUndefined();
  });

  it("never throws when the storage cannot be used", () => {
    const store = new PlayerStorage("p", new BrokenStorage());
    expect(store.get(STORAGE_NAMES.lighting)).toBeNull();
    expect(store.getJson(STORAGE_NAMES.lighting)).toBeUndefined();
    expect(store.getToggle(STORAGE_NAMES.autoIdle, true)).toBe(true);
    expect(() => store.set(STORAGE_NAMES.lighting, "x")).not.toThrow();
    expect(() => store.setToggle(STORAGE_NAMES.autoIdle, false)).not.toThrow();
  });

  it("uses localStorage when no storage is given, even if reading it throws", () => {
    const storage = new MemoryStorage();
    vi.stubGlobal("localStorage", storage);
    const store = new PlayerStorage();
    store.set(STORAGE_NAMES.rememberedOrder, "name");
    expect(storage.getItem("ugoe.viewer.remembered-order")).toBe("name");

    // 引くだけで例外になる環境。
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      get() {
        throw new Error("SecurityError");
      },
    });
    expect(store.get(STORAGE_NAMES.rememberedOrder)).toBeNull();
    expect(() => store.set(STORAGE_NAMES.rememberedOrder, "recent")).not.toThrow();
    // `unstubAllGlobals` が元に戻せるように、値の属性に戻す。
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      writable: true,
      value: storage,
    });
  });
});
