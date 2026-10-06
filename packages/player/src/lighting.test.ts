import { setLocale } from "@ugoe/i18n";
import { MemoryStorage } from "@ugoe/i18n/testing";
import { beforeEach, describe, expect, it } from "vitest";
import {
  anglesFromPad,
  anglesFromScreen,
  defaultLighting,
  directionalHandle,
  type LightingSettings,
  lightDial,
  lightingLabel,
  lightingMemory,
  loadLighting,
  MAX_LIGHTING_MODELS,
  PRESETS,
  padFromAngles,
  pointFromScreen,
  pointHandle,
  pointHandleOnScreen,
  pointRangeOnCanvas,
  rangeFromScreenRadius,
  rangeGrip,
  rangeRadiusFromGrip,
  sameLighting,
  sanitizeLighting,
  saveLighting,
  stepMarker,
  toLightingJson,
  towardLight,
  wheelPointZ,
} from "./lighting";
import { type ModelIdentity, modelIdentity } from "./modelMemory";
import { PlayerStorage } from "./storage";

/** 前からのビューアの接頭辞で `storage` に覚える入れ物。 */
const store = (storage: Storage) => new PlayerStorage(undefined, storage);

// 文言は日本語の訳で確かめる (英語の原文を確かめる it は、その中で言語を切り替える)。
beforeEach(() => setLocale("ja"));

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
    saveLighting(s, store(storage));
    const loaded = loadLighting(store(storage));
    expect(loaded).toEqual(sanitizeLighting(s));
    expect(sameLighting(loaded, s)).toBe(true);
  });

  it("falls back to the default for broken or foreign data", () => {
    const storage = new MemoryStorage();
    expect(loadLighting(store(storage))).toEqual(defaultLighting());
    storage.setItem("ugoe.viewer.lighting", "{not json");
    expect(loadLighting(store(storage))).toEqual(defaultLighting());
    storage.setItem("ugoe.viewer.lighting", JSON.stringify({ version: 99, settings: {} }));
    expect(loadLighting(store(storage))).toEqual(defaultLighting());
  });

  it("survives a storage that throws", () => {
    const storage = new MemoryStorage();
    storage.getItem = () => {
      throw new Error("denied");
    };
    storage.setItem = () => {
      throw new Error("quota");
    };
    expect(loadLighting(store(storage))).toEqual(defaultLighting());
    expect(() => saveLighting(defaultLighting(), store(storage))).not.toThrow();
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
    const memory = lightingMemory(store(storage));
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
    const memory = lightingMemory(store(storage));
    for (let n = 0; n <= MAX_LIGHTING_MODELS; n++) memory.set(model(n), sunset());
    expect(memory.models()).toHaveLength(MAX_LIGHTING_MODELS);
    expect(memory.get(model(0))).toBeNull();
    expect(memory.get(model(1))).not.toBeNull();
  });

  it("repairs broken settings and skips entries that are not settings", () => {
    const storage = new MemoryStorage();
    const m = model(1);
    storage.setItem(
      "ugoe.viewer.lighting.models",
      JSON.stringify({
        version: 1,
        entries: [
          { model: model(2), value: "nope" },
          { model: model(3), value: [1, 2] },
          { model: m, value: { ambient: { color: "nope", intensity: "x" }, point: { x: 1e99 } } },
        ],
      }),
    );
    const memory = lightingMemory(store(storage));
    expect(memory.get(model(2))).toBeNull();
    expect(memory.get(model(3))).toBeNull();
    const repaired = memory.get(m);
    expect(repaired?.ambient).toEqual(defaultLighting().ambient);
    expect(repaired?.point.x).toBe(1.5);
    storage.setItem("ugoe.viewer.lighting.models", "{not json");
    expect(memory.get(m)).toBeNull();
    expect(() => memory.set(m, defaultLighting())).not.toThrow();
  });

  it("does not touch the last used settings", () => {
    const storage = new MemoryStorage();
    saveLighting(sunset(), store(storage));
    lightingMemory(store(storage)).set(model(1), defaultLighting());
    expect(sameLighting(loadLighting(store(storage)), sunset())).toBe(true);
  });
});

describe("lightingLabel", () => {
  it("names presets, custom settings and lights off", () => {
    expect(lightingLabel(defaultLighting())).toBe("標準");
    const custom = defaultLighting();
    custom.ambient.intensity = 0.1;
    expect(lightingLabel(custom)).toBe("カスタム");
    expect(lightingLabel({ ...defaultLighting(), enabled: false })).toBe("切");
    setLocale("en");
    expect(lightingLabel(defaultLighting())).toBe("Standard");
    expect(lightingLabel({ ...defaultLighting(), enabled: false })).toBe("Off");
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
    expect(lightDial(canvas, view)).toEqual({
      cx: 200,
      cy: 250,
      r: 70,
      shifted: false,
      canvasCenter: [200, 250],
    });
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

  it("大きく拡大して円が画面からはみ出すときは、円ごと画面の内側へ寄せる (T0714)", () => {
    const bounds = [800, 600] as const;
    // 画面に収まっていれば動かさない。
    expect(lightDial(canvas, view, bounds)).toMatchObject({
      cx: 200,
      cy: 250,
      r: 70,
      shifted: false,
    });
    // 10 倍に拡大してキャンバスの中央 (2000 + 100, 4000 + 50) が画面の外: 端から半径 + 余白の位置に寄せる。
    const zoomed = lightDial(canvas, { scale: 10, x: 100, y: 50 }, bounds);
    expect(zoomed.r).toBe(220);
    expect(zoomed.cx).toBe(800 - 220 - 14);
    expect(zoomed.cy).toBe(600 - 220 - 14);
    // 寄せたことと、本当のキャンバスの中央が分かる (印の見た目を変える、T0717)。
    expect(zoomed.shifted).toBe(true);
    expect(zoomed.canvasCenter).toEqual([2100, 4050]);
    // 画面が円より狭ければ画面の中央。
    expect(lightDial(canvas, { scale: 10, x: -5000, y: 50 }, [300, 300])).toMatchObject({
      cx: 150,
      cy: 150,
    });
  });

  it("点光源の取っ手は、画面の外なら画面の縁に寄せる (T0714)", () => {
    const p = { ...defaultLighting().point, x: 0.25, y: 0.5 };
    expect(pointHandleOnScreen(canvas, view, p, [800, 600])).toEqual({
      x: 150,
      y: 250,
      offscreen: false,
    });
    expect(pointHandleOnScreen(canvas, view, { ...p, x: 1.5, y: -0.5 }, [200, 200])).toEqual({
      x: 200 - 14,
      y: 14,
      offscreen: true,
    });
  });

  it("範囲の円の縁が画面に見えないときだけ、画面の端に範囲の取っ手を出す (T0717・T0718)", () => {
    const bounds = [800, 600] as const;
    const onRay = (g: ReturnType<typeof rangeGrip>, center: readonly [number, number]) => {
      // 取っ手は光源から伸ばした dir の線の上にある。
      const along =
        ((g?.x ?? 0) - center[0]) * (g?.dir[0] ?? 0) + ((g?.y ?? 0) - center[1]) * (g?.dir[1] ?? 0);
      expect(Math.hypot((g?.x ?? 0) - center[0], (g?.y ?? 0) - center[1])).toBeCloseTo(along);
    };
    // 縁が画面を通る → 出さない (縁を直接掴む)。
    expect(rangeGrip([400, 300], 200, bounds)).toBeNull();
    // 画面をすっぽり覆う → 光源からいちばん遠い角 (縁が最初に入ってくるところ)。
    const cover = rangeGrip([200, 200], 2000, bounds);
    expect(cover?.x).toBe(800 - 14);
    expect(cover?.y).toBe(600 - 14);
    expect(cover?.dir[0]).toBeGreaterThan(0);
    expect(cover?.dir[1]).toBeGreaterThan(0);
    onRay(cover, [200, 200]);
    // 光源が画面の中央に近くても、右向きに固定しない。中央のわずかに右下なら左上の角。
    const nearCenter = rangeGrip([402, 303], 2000, bounds);
    expect([nearCenter?.x, nearCenter?.y]).toEqual([14, 14]);
    expect(nearCenter?.dir[0]).toBeLessThan(0);
    expect(nearCenter?.dir[1]).toBeLessThan(0);
    onRay(nearCenter, [402, 303]);
    // 中央ちょうど (4 つの角が同じ遠さ) なら右下。
    const center = rangeGrip([400, 300], 2000, bounds);
    expect([center?.x, center?.y]).toEqual([786, 586]);
    // まるごと画面の外 (左上の外の小さな円) → 画面の左上の角 (光源にいちばん近い画面の点)。
    const outside = rangeGrip([-400, -300], 100, bounds);
    expect(outside?.x).toBe(14);
    expect(outside?.y).toBe(14);
    expect(outside?.dir[0]).toBeCloseTo(414 / Math.hypot(414, 314));
    expect(outside?.dir[1]).toBeCloseTo(314 / Math.hypot(414, 314));
    // 左の外 (上下は画面の中) → 左の縁の、光源と同じ高さ。向きは真右。
    const left = rangeGrip([-500, 100], 200, bounds);
    expect([left?.x, left?.y]).toEqual([14, 100]);
    expect(left?.dir).toEqual([1, 0]);
    // 範囲が無い・画面が狭すぎる → 出さない。
    expect(rangeGrip([200, 300], 0, bounds)).toBeNull();
    expect(rangeGrip([200, 300], 2000, [20, 20])).toBeNull();
  });

  it("範囲の取っ手を向きに沿って動かした分だけ半径が変わる (T0717)", () => {
    // 右向きの取っ手: 右へ 30 で +30、上下の動きは効かない、左へ動かしすぎても 0 より小さくならない。
    expect(rangeRadiusFromGrip(500, [1, 0], [786, 300], [816, 250])).toBe(530);
    expect(rangeRadiusFromGrip(500, [1, 0], [786, 300], [0, 300])).toBe(0);
    expect(rangeRadiusFromGrip(100, [0.8, 0.6], [14, 14], [22, 20])).toBeCloseTo(110);
  });

  it("範囲の円の縁を動かすと、手前を保ったまま範囲が変わる (T0714)", () => {
    const p = { ...defaultLighting().point, z: 0.3, range: 0.5 };
    // 面の上の半径 320 (画面では × 0.5 = 160) の範囲は 0.5。
    expect(rangeFromScreenRadius(canvas, view, p, 160)).toBeCloseTo(0.5, 3);
    // 半径 0 → 範囲は手前の距離と同じ (範囲の下限 0.1 より大きい)。
    expect(rangeFromScreenRadius(canvas, view, p, 0)).toBeCloseTo(0.3, 3);
    expect(rangeFromScreenRadius(canvas, view, p, 1e6)).toBe(4);
    // 往復。
    const r = pointRangeOnCanvas(canvas, { ...p, range: 1.2 }) * view.scale;
    expect(rangeFromScreenRadius(canvas, view, p, r)).toBeCloseTo(1.2, 3);
  });

  it("取っ手をキーボードで動かす (T0714)", () => {
    const next = defaultLighting();
    // 平行光源: → で取っ手が右へ (方位が 0° に近づく)。円の外へは出ない。
    const before = padFromAngles(next.directional.azimuth, next.directional.elevation);
    expect(stepMarker(next, "directional", "ArrowRight", false)).toBe(true);
    const after = padFromAngles(next.directional.azimuth, next.directional.elevation);
    expect(after[0]).toBeCloseTo(before[0] + 0.05, 2);
    expect(after[1]).toBeCloseTo(before[1], 2);
    for (let i = 0; i < 20; i++) stepMarker(next, "directional", "ArrowRight", true);
    expect(next.directional.elevation).toBeCloseTo(0, 0);
    // 縁に沿って右へ寄っていく。
    expect(Math.abs(next.directional.azimuth)).toBeLessThan(5);
    expect(stepMarker(next, "directional", "PageUp", false)).toBe(false);

    // 点光源: 矢印で横・縦 (Shift で 10%)、PageUp / PageDown で手前、+ / − で範囲。範囲に収める。
    const p = next.point;
    const { x, y, z, range } = p;
    stepMarker(next, "point", "ArrowRight", false);
    stepMarker(next, "point", "ArrowDown", true);
    expect(p.x).toBeCloseTo(x + 0.01, 6);
    expect(p.y).toBeCloseTo(y + 0.1, 6);
    stepMarker(next, "point", "PageUp", false);
    expect(p.z).toBeCloseTo(z + 0.02, 6);
    stepMarker(next, "point", "+", false);
    stepMarker(next, "point", "=", false);
    stepMarker(next, "point", "-", false);
    expect(p.range).toBeCloseTo(range + 0.05, 6);
    for (let i = 0; i < 100; i++) stepMarker(next, "point", "PageDown", true);
    expect(p.z).toBe(0);
    expect(stepMarker(next, "point", "a", false)).toBe(false);
  });

  it("Shift + ホイールで点光源の手前を変える (T0714)", () => {
    const p = { ...defaultLighting().point, z: 0.4 };
    expect(wheelPointZ(p, -100)).toBeCloseTo(0.42, 6);
    expect(wheelPointZ(p, 100)).toBeCloseTo(0.38, 6);
    expect(wheelPointZ(p, 0)).toBe(0.4);
    expect(wheelPointZ({ ...p, z: 2 }, -1)).toBe(2);
  });
});
