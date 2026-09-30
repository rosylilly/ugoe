import { describe, expect, it } from "vitest";
import {
  anglesFromPad,
  defaultLighting,
  type LightingSettings,
  lightingMemory,
  loadLighting,
  MAX_LIGHTING_MODELS,
  MODEL_LIGHTING_STORAGE_KEY,
  PRESETS,
  padFromAngles,
  sameLighting,
  sanitizeLighting,
  saveLighting,
  toLightingJson,
  towardLight,
} from "./lighting";
import { type ModelIdentity, modelIdentity } from "./modelMemory";

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

function closeTo(actual: number[], expected: number[]) {
  expect(actual).toHaveLength(expected.length);
  actual.forEach((v, i) => {
    expect(v).toBeCloseTo(expected[i] ?? Number.NaN, 5);
  });
}

describe("towardLight", () => {
  it("points to where the light comes from in canvas coordinates", () => {
    closeTo(towardLight(0, 0), [1, 0, 0]); // 右から
    closeTo(towardLight(90, 0), [0, -1, 0]); // 上から (Y 下向き)
    closeTo(towardLight(180, 0), [-1, 0, 0]);
    closeTo(towardLight(30, 90), [0, 0, 1]); // 真正面
  });
});

describe("toLightingJson", () => {
  it("turns angles into the direction the light travels and scales colors", () => {
    const s = defaultLighting();
    s.ambient = { color: "#ff8000", intensity: 0.5 };
    s.directional = { enabled: true, color: "#ffffff", intensity: 2, azimuth: 180, elevation: 0 };
    s.point.enabled = false;
    const json = toLightingJson(s, [100, 200]);
    expect(json?.ambient).toEqual([0.5, (128 / 255) * 0.5, 0]);
    expect(json?.lights).toHaveLength(1);
    const light = json?.lights[0];
    expect(light?.type).toBe("directional");
    if (light?.type !== "directional") return;
    // 左から来る光は右へ進む。
    closeTo(light.direction, [1, 0, 0]);
    expect(light.color).toEqual([2, 2, 2]);
  });

  it("places point lights relative to the canvas", () => {
    const s = defaultLighting();
    s.directional.enabled = false;
    s.point = {
      enabled: true,
      color: "#ffffff",
      intensity: 1,
      x: 0.25,
      y: 0.5,
      z: 0.5,
      range: 2,
    };
    expect(toLightingJson(s, [400, 800])?.lights).toEqual([
      { type: "point", position: [100, 400, 400], color: [1, 1, 1], range: 1600 },
    ]);
  });

  it("returns null when the lights are off", () => {
    const s = defaultLighting();
    s.enabled = false;
    expect(toLightingJson(s, [1, 1])).toBeNull();
  });

  it("builds valid lights for every preset", () => {
    for (const preset of PRESETS) {
      const json = toLightingJson(preset.settings, [512, 512]);
      expect(json).not.toBeNull();
      expect(json?.lights.length).toBeGreaterThan(0);
      for (const c of json?.ambient ?? []) expect(c).toBeGreaterThanOrEqual(0);
    }
  });
});

describe("pad", () => {
  it("round-trips angles through the pad position", () => {
    for (const [az, el] of [
      [135, 45],
      [-90, 10],
      [0, 0],
      [60, 80],
    ] as const) {
      const [x, y] = padFromAngles(az, el);
      closeTo(anglesFromPad(x, y), [az, el]);
    }
  });

  it("treats the center as straight ahead and clamps outside the circle", () => {
    expect(anglesFromPad(0, 0)).toEqual([0, 90]);
    closeTo(anglesFromPad(0, -2), [90, 0]);
  });
});

describe("sanitizeLighting", () => {
  it("fills missing values and clamps out-of-range ones", () => {
    const s = sanitizeLighting({
      enabled: false,
      ambient: { color: "red", intensity: 99 },
      directional: { azimuth: "left", elevation: -5 },
      point: { x: Number.NaN, range: 0 },
    });
    const base = defaultLighting();
    expect(s.enabled).toBe(false);
    expect(s.ambient).toEqual({ color: base.ambient.color, intensity: 3 });
    expect(s.directional.azimuth).toBe(base.directional.azimuth);
    expect(s.directional.elevation).toBe(0);
    expect(s.point.x).toBe(base.point.x);
    expect(s.point.range).toBe(0.1);
  });

  it("ignores things that are not objects", () => {
    for (const value of [null, 3, "x", [1, 2]]) {
      expect(sanitizeLighting(value)).toEqual(defaultLighting());
    }
  });
});

describe("loadLighting / saveLighting", () => {
  it("round-trips through storage", () => {
    const storage = new MemoryStorage();
    const s: LightingSettings = structuredClone(PRESETS[2]?.settings) as LightingSettings;
    s.point.x = 0.9;
    saveLighting(s, storage);
    const loaded = loadLighting(storage);
    expect(loaded).toEqual(sanitizeLighting(s));
    expect(sameLighting(loaded, s)).toBe(true);
  });

  it("falls back to the default for broken or foreign data", () => {
    const storage = new MemoryStorage();
    expect(loadLighting(storage)).toEqual(defaultLighting());
    storage.setItem("ugoe-viewer.lighting", "{not json");
    expect(loadLighting(storage)).toEqual(defaultLighting());
    storage.setItem("ugoe-viewer.lighting", JSON.stringify({ version: 99, settings: {} }));
    expect(loadLighting(storage)).toEqual(defaultLighting());
  });

  it("survives a storage that throws", () => {
    const storage = new MemoryStorage();
    storage.getItem = () => {
      throw new Error("denied");
    };
    storage.setItem = () => {
      throw new Error("quota");
    };
    expect(loadLighting(storage)).toEqual(defaultLighting());
    expect(() => saveLighting(defaultLighting(), storage)).not.toThrow();
  });
});

describe("lightingMemory", () => {
  const model = (n: number, location?: string): ModelIdentity =>
    modelIdentity(
      location === undefined
        ? { name: `m${n}.ugoe`, bytes: new Uint8Array([n, n >> 8]) }
        : { name: `m${n}.ugoe`, bytes: new Uint8Array([n, n >> 8]), location: `path:${location}` },
    );
  const sunset = () => structuredClone(PRESETS[1]?.settings) as LightingSettings;

  it("remembers settings per model and forgets them", () => {
    const storage = new MemoryStorage();
    const memory = lightingMemory(storage);
    memory.set(model(1), sunset());
    expect(memory.get(model(1))).toEqual(sanitizeLighting(sunset()));
    expect(memory.get(model(2))).toBeNull();
    // 同じ中身を場所付きで開いても同じモデル。書き出し直して中身が変わっても、場所が同じなら同じモデル。
    memory.set(model(3, "/a.ugoe"), defaultLighting());
    expect(memory.get(model(4, "/a.ugoe"))).toEqual(defaultLighting());
    memory.delete(model(1));
    expect(memory.get(model(1))).toBeNull();
    expect(memory.get(model(3))).toEqual(defaultLighting());
  });

  it("keeps only the most recently used models", () => {
    const storage = new MemoryStorage();
    const memory = lightingMemory(storage);
    for (let n = 0; n <= MAX_LIGHTING_MODELS; n++) memory.set(model(n), sunset());
    expect(memory.models()).toHaveLength(MAX_LIGHTING_MODELS);
    expect(memory.get(model(0))).toBeNull();
    expect(memory.get(model(1))).not.toBeNull();
  });

  it("repairs broken settings and skips entries that are not settings", () => {
    const storage = new MemoryStorage();
    const m = model(1);
    storage.setItem(
      MODEL_LIGHTING_STORAGE_KEY,
      JSON.stringify({
        version: 1,
        entries: [
          { model: model(2), value: "nope" },
          { model: model(3), value: [1, 2] },
          { model: m, value: { ambient: { color: "nope", intensity: "x" }, point: { x: 1e99 } } },
        ],
      }),
    );
    const memory = lightingMemory(storage);
    expect(memory.get(model(2))).toBeNull();
    expect(memory.get(model(3))).toBeNull();
    const repaired = memory.get(m);
    expect(repaired?.ambient).toEqual(defaultLighting().ambient);
    expect(repaired?.point.x).toBe(1.5);
    storage.setItem(MODEL_LIGHTING_STORAGE_KEY, "{not json");
    expect(memory.get(m)).toBeNull();
    expect(() => memory.set(m, defaultLighting())).not.toThrow();
  });

  it("does not touch the last used settings", () => {
    const storage = new MemoryStorage();
    saveLighting(sunset(), storage);
    lightingMemory(storage).set(model(1), defaultLighting());
    expect(sameLighting(loadLighting(storage), sunset())).toBe(true);
  });
});

describe("sameLighting", () => {
  it("ignores field order", () => {
    const a = defaultLighting();
    const b = sanitizeLighting(JSON.parse(JSON.stringify(a)));
    expect(sameLighting(a, b)).toBe(true);
    b.directional.azimuth += 1;
    expect(sameLighting(a, b)).toBe(false);
  });
});
