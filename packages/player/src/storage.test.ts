import { afterEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_STORAGE_PREFIX, PlayerStorage, STORAGE_NAMES } from "./storage";

/** 最小の Storage (Map に覚える)。 */
function memoryStorage(): Storage & { items: Map<string, string> } {
  const items = new Map<string, string>();
  return {
    items,
    getItem: (k) => items.get(k) ?? null,
    setItem: (k, v) => void items.set(k, v),
    removeItem: (k) => void items.delete(k),
    clear: () => items.clear(),
    key: (i) => [...items.keys()][i] ?? null,
    get length() {
      return items.size;
    },
  };
}

/** 読み書きのたびに例外を出す Storage (保存を禁止したブラウザ・容量の超過)。 */
const broken = {
  getItem: () => {
    throw new Error("denied");
  },
  setItem: () => {
    throw new Error("quota");
  },
  removeItem: () => {
    throw new Error("denied");
  },
} as unknown as Storage;

afterEach(() => vi.unstubAllGlobals());

describe("PlayerStorage", () => {
  it("keeps the viewer's keys when the prefix is omitted", () => {
    const storage = memoryStorage();
    const store = new PlayerStorage(undefined, storage);
    expect(store.prefix).toBe(DEFAULT_STORAGE_PREFIX);
    // 前からのビューアのキー (接頭辞を渡せるようにする前に覚えたものを読めるように)。
    expect(Object.values(STORAGE_NAMES).map((name) => store.key(name))).toEqual([
      "ugoe-viewer.lighting",
      "ugoe-viewer.lighting.models",
      "ugoe-viewer.lighting.markers",
      "ugoe-viewer.dress-up",
      "ugoe-viewer.dress-up.auto-restore",
      "ugoe-viewer.idle-motion.models",
      "ugoe-viewer.auto-idle",
      "ugoe-viewer.motion-over-tracking",
      "ugoe-viewer.motion-over-tracking.kept",
      "ugoe-viewer.remembered-order",
    ]);
  });

  it("reads and writes only under its own prefix", () => {
    const storage = memoryStorage();
    const viewer = new PlayerStorage(undefined, storage);
    const studio = new PlayerStorage("ugoe-studio.player", storage);
    viewer.set(STORAGE_NAMES.rememberedOrder, "name");
    expect(studio.get(STORAGE_NAMES.rememberedOrder)).toBeNull();
    studio.setJson(STORAGE_NAMES.trackingKept, ["EyeLOpen"]);
    expect(storage.items.get("ugoe-studio.player.motion-over-tracking.kept")).toBe('["EyeLOpen"]');
    expect(viewer.getJson(STORAGE_NAMES.trackingKept)).toBeUndefined();
    expect(studio.getJson(STORAGE_NAMES.trackingKept)).toEqual(["EyeLOpen"]);
  });

  it("remembers on and off and falls back otherwise", () => {
    const store = new PlayerStorage("p", memoryStorage());
    expect(store.getToggle(STORAGE_NAMES.autoIdle, true)).toBe(true);
    store.setToggle(STORAGE_NAMES.autoIdle, false);
    expect(store.getToggle(STORAGE_NAMES.autoIdle, true)).toBe(false);
    store.setToggle(STORAGE_NAMES.autoIdle, true);
    expect(store.getToggle(STORAGE_NAMES.autoIdle, false)).toBe(true);
    store.set(STORAGE_NAMES.autoIdle, "maybe");
    expect(store.getToggle(STORAGE_NAMES.autoIdle, false)).toBe(false);
  });

  it("treats broken JSON as nothing remembered", () => {
    const store = new PlayerStorage("p", memoryStorage());
    store.set(STORAGE_NAMES.lighting, "{not json");
    expect(store.getJson(STORAGE_NAMES.lighting)).toBeUndefined();
  });

  it("forgets everything under its prefix and nothing else", () => {
    const storage = memoryStorage();
    const old = new PlayerStorage("ugoe-viewer", storage);
    for (const name of Object.values(STORAGE_NAMES)) old.set(name, "x");
    storage.setItem("ugoe-viewer.locale", "ja");
    new PlayerStorage("ugoe-studio.player", storage).set(STORAGE_NAMES.lighting, "y");
    old.clear();
    expect([...storage.items.keys()].sort()).toEqual([
      "ugoe-studio.player.lighting",
      "ugoe-viewer.locale",
    ]);
  });

  it("never throws when the storage cannot be used", () => {
    const store = new PlayerStorage("p", broken);
    expect(store.get(STORAGE_NAMES.lighting)).toBeNull();
    expect(store.getJson(STORAGE_NAMES.lighting)).toBeUndefined();
    expect(store.getToggle(STORAGE_NAMES.autoIdle, true)).toBe(true);
    expect(() => store.set(STORAGE_NAMES.lighting, "x")).not.toThrow();
    expect(() => store.setToggle(STORAGE_NAMES.autoIdle, false)).not.toThrow();
    expect(() => store.clear()).not.toThrow();
  });

  it("uses localStorage when no storage is given, even if reading it throws", () => {
    const storage = memoryStorage();
    vi.stubGlobal("localStorage", storage);
    const store = new PlayerStorage();
    store.set(STORAGE_NAMES.rememberedOrder, "name");
    expect(storage.items.get("ugoe-viewer.remembered-order")).toBe("name");

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
