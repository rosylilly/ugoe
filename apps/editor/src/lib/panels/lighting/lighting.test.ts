import { describe, expect, it } from "vitest";
import type { Model, NormalMapGroupView, PreviewLighting } from "../../ipc";
import {
  bulgeForm,
  bulgeLimits,
  bulgeSettings,
  bulgeUnit,
  DEFAULT_PREVIEW_LIGHTING,
  directionToHandle,
  elevationDegrees,
  handleToDirection,
  hexToRgb,
  lightingHint,
  lightingJson,
  newLight,
  rangeOnCanvas,
  rgbToHex,
} from "./lighting";

describe("lightingJson", () => {
  it("multiplies colors by the intensity and omits an unlimited range", () => {
    const lighting: PreviewLighting = {
      enabled: true,
      ambient: [1, 0.5, 0],
      ambient_intensity: 0.5,
      lights: [
        { type: "directional", direction: [1, 1, -1], color: [1, 1, 1], intensity: 2 },
        { type: "point", position: [10, 20, 30], color: [1, 0, 0], intensity: 1, range: null },
        { type: "point", position: [0, 0, 5], color: [0, 1, 0], intensity: 0.5, range: 100 },
      ],
    };
    expect(JSON.parse(lightingJson(lighting) ?? "")).toEqual({
      ambient: [0.5, 0.25, 0],
      lights: [
        { type: "directional", direction: [1, 1, -1], color: [2, 2, 2] },
        { type: "point", position: [10, 20, 30], color: [1, 0, 0] },
        { type: "point", position: [0, 0, 5], color: [0, 0.5, 0], range: 100 },
      ],
    });
  });

  it("passes no lights when disabled", () => {
    expect(lightingJson({ ...DEFAULT_PREVIEW_LIGHTING, enabled: false })).toBeNull();
  });
});

describe("directional handle", () => {
  it("points at the light: top-left front for (1, 1, -1)", () => {
    const h = directionToHandle([1, 1, -1]);
    expect(h.x).toBeCloseTo(-1 / Math.sqrt(3));
    expect(h.y).toBeCloseTo(-1 / Math.sqrt(3));
    expect(h.behind).toBe(false);
  });

  it("round-trips through the handle", () => {
    for (const d of [
      [0.3, -0.2, -0.93],
      [-0.6, 0.1, 0.79],
      [0, 0, -1],
    ] as [number, number, number][]) {
      const h = directionToHandle(d);
      const back = handleToDirection(h.x, h.y, h.behind);
      const len = Math.hypot(...d);
      for (const [i, v] of back.entries()) expect(v).toBeCloseTo((d[i] ?? 0) / len);
    }
  });

  it("clamps a handle outside the circle to a light from the side", () => {
    const d = handleToDirection(3, 4, false);
    expect(d[0]).toBeCloseTo(-0.6);
    expect(d[1]).toBeCloseTo(-0.8);
    expect(d[2]).toBeCloseTo(0);
  });

  it("measures the elevation from the canvas plane", () => {
    expect(elevationDegrees([0, 0, -1])).toBeCloseTo(90);
    expect(elevationDegrees([1, 0, 0])).toBeCloseTo(0);
    expect(elevationDegrees([0, 1, 1])).toBeCloseTo(-45);
  });
});

describe("point lights", () => {
  it("reaches the canvas within sqrt(range² − z²)", () => {
    expect(
      rangeOnCanvas({
        type: "point",
        position: [0, 0, 30],
        color: [1, 1, 1],
        intensity: 1,
        range: 50,
      }),
    ).toBeCloseTo(40);
    expect(
      rangeOnCanvas({
        type: "point",
        position: [0, 0, 80],
        color: [1, 1, 1],
        intensity: 1,
        range: 50,
      }),
    ).toBe(0);
    expect(
      rangeOnCanvas({
        type: "point",
        position: [0, 0, 1],
        color: [1, 1, 1],
        intensity: 1,
        range: null,
      }),
    ).toBeNull();
  });

  it("places a new point light in front of the canvas", () => {
    const light = newLight("point", { width: 1000, height: 500 });
    expect(light).toMatchObject({ type: "point", position: [500, 500 / 3, 300], range: 1200 });
  });
});

describe("colors", () => {
  it("converts to and from hex", () => {
    expect(rgbToHex([1, 0.5, 0])).toBe("#ff8000");
    expect(rgbToHex([2, -1, 1])).toBe("#ff00ff");
    expect(hexToRgb("#ff8000")).toEqual([1, 128 / 255, 0]);
    expect(hexToRgb("red")).toBeNull();
  });
});

describe("lightingHint", () => {
  const mesh = (lit: boolean, mask_only = false) => ({ lit, mask_only }) as Model["meshes"][number];
  it("explains why lights do nothing", () => {
    expect(lightingHint({ lighting: false, meshes: [mesh(true)] } as Model)).toMatch(
      /off for this model/,
    );
    expect(
      lightingHint({ lighting: true, meshes: [mesh(false), mesh(true, true)] } as Model),
    ).toMatch(/No mesh/);
    expect(lightingHint({ lighting: true, meshes: [mesh(true)] } as Model)).toBeNull();
    // lit は既定 true で、JSON では省略される。
    expect(lightingHint({ lighting: true, meshes: [{ mask_only: false }] } as Model)).toBeNull();
    expect(lightingHint({ meshes: [mesh(true)] } as unknown as Model)).toMatch(
      /off for this model/,
    );
  });
});

describe("bulge settings", () => {
  const view = (canvasPerPixel: number | null, edge: number | null): NormalMapGroupView => ({
    group: canvasPerPixel === null ? null : "face",
    textures: ["face"],
    shape_only: [],
    oriented: [],
    with_normal_map: [],
    defaults: { strength: 0.35, edge, smoothing: 6 },
    canvas_per_pixel: canvasPerPixel,
    from_classification: false,
  });

  it("shows lengths in canvas px for a group and converts them back", () => {
    // 格子の 1 画素がキャンバスの 0.25 px: 格子の 192 画素は 48 px。
    const face = view(0.25, 192);
    const form = bulgeForm(face);
    expect(bulgeUnit(face)).toBe("px");
    expect(form.edge).toBeCloseTo(48);
    expect(form.smoothing).toBeCloseTo(1.5);
    const settings = bulgeSettings({ ...form, edge: 40 }, face);
    expect(settings.edge).toBeCloseTo(160);
    expect(settings.smoothing).toBeCloseTo(6);
    expect(bulgeSettings({ ...form, roundEdges: false }, face).edge).toBeNull();
    expect(bulgeLimits(face)).toEqual({ edge: 96, smoothing: 10 });
  });

  it("keeps texture pixels for parts shaped on their own", () => {
    const alone = view(null, null);
    expect(bulgeUnit(alone)).toBe("texture px");
    expect(bulgeForm(alone)).toEqual({ strength: 0.35, roundEdges: false, edge: 12, smoothing: 6 });
    expect(bulgeLimits(alone)).toEqual({ edge: 64, smoothing: 12 });
  });
});
