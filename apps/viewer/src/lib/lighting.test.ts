import { describe, expect, it } from "vitest";
import {
  anglesFromPad,
  anglesFromScreen,
  defaultLighting,
  directionalHandle,
  type LightingSettings,
  lightDial,
  lightingMemory,
  loadLighting,
  loadLightMarkers,
  MAX_LIGHTING_MODELS,
  MODEL_LIGHTING_STORAGE_KEY,
  PRESETS,
  padFromAngles,
  pointFromScreen,
  pointHandle,
  pointRangeOnCanvas,
  sameLighting,
  sanitizeLighting,
  saveLighting,
  saveLightMarkers,
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

describe("ビューポートの光源の印", () => {
  const canvas = [400, 800] as const;
  // 半分に縮めて (100, 50) にずらした表示。
  const view = { scale: 0.5, x: 100, y: 50 };

  it("平行光源の円はキャンバスの中央に置き、半径は短い辺の 35% を範囲に収める", () => {
    expect(lightDial(canvas, view)).toEqual({ cx: 200, cy: 250, r: 70 });
    expect(lightDial(canvas, { ...view, scale: 0.1 }).r).toBe(60);
    expect(lightDial(canvas, { ...view, scale: 5 }).r).toBe(220);
  });

  it("平行光源の取っ手は光が来る向きにあり、動かした位置から角度に戻せる", () => {
    const dial = lightDial(canvas, view);
    const d = { ...defaultLighting().directional, azimuth: 180, elevation: 0 };
    closeTo(directionalHandle(dial, d), [130, 250]);
    // 真上の縁 → 上から真横に、中心 → 正面から、円の外 → 縁に収める。
    expect(anglesFromScreen(dial, 200, 180)).toEqual([90, 0]);
    expect(anglesFromScreen(dial, 200, 250)[1]).toBe(90);
    // (-0 と 0 は区別しない)
    expect(anglesFromScreen(dial, 200 + 700, 250).map((v) => v + 0)).toEqual([0, 0]);
    const [azimuth, elevation] = anglesFromScreen(
      dial,
      ...directionalHandle(dial, {
        ...d,
        azimuth: 135,
        elevation: 45,
      }),
    );
    expect(azimuth).toBeCloseTo(135, 0);
    expect(elevation).toBeCloseTo(45, 0);
  });

  it("点光源の取っ手はキャンバスの上の位置にあり、画面の位置から割合に戻せる", () => {
    const p = { ...defaultLighting().point, x: 0.25, y: 0.5 };
    expect(pointHandle(canvas, view, p)).toEqual([150, 250]);
    expect(pointFromScreen(canvas, view, 150, 250)).toEqual([0.25, 0.5]);
    // 範囲の外は収める。
    expect(pointFromScreen(canvas, view, -10000, 10000)).toEqual([-0.5, 1.5]);
  });

  it("点光源の光が面の上で届く範囲", () => {
    const p = { ...defaultLighting().point, z: 0.3, range: 0.5 };
    // 長い辺 800: 範囲 400、手前 240 → √(400² − 240²) = 320。
    expect(pointRangeOnCanvas(canvas, p)).toBeCloseTo(320);
    expect(pointRangeOnCanvas(canvas, { ...p, z: 1 })).toBe(0);
  });

  it("印を出すかを覚える (既定は出す。読めなければ出す)", () => {
    const storage = new MemoryStorage();
    expect(loadLightMarkers(storage)).toBe(true);
    saveLightMarkers(false, storage);
    expect(loadLightMarkers(storage)).toBe(false);
    saveLightMarkers(true, storage);
    expect(loadLightMarkers(storage)).toBe(true);
    const broken = {
      getItem() {
        throw new Error("denied");
      },
    } as unknown as Storage;
    expect(loadLightMarkers(broken)).toBe(true);
  });
});
