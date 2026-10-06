import { BrokenStorage, MemoryStorage, useLocalStorage } from "@ugoe/i18n/testing";
import { describe, expect, it } from "vitest";
import type { Curve, Motion, Parameter } from "../../ipc";
import {
  advance,
  canMoveKey,
  clampHandle,
  curveKeys,
  curvePath,
  followKey,
  isValidName,
  KeyMoves,
  keyValuesCommand,
  loadPanelOpen,
  nearestMark,
  newMotion,
  savePanelOpen,
  segmentEndingAt,
  segmentHandle,
  segmentShape,
  shapedKeys,
  snapToFrame,
  tickStep,
  timeToX,
  tracks,
  uniqueName,
  valueRange,
  valueToY,
  xToTime,
  yToValue,
} from "./timeline";

function param(id: string, min = -30, def = 0, max = 30): Parameter {
  return {
    id,
    name: id,
    min,
    default: def,
    max,
    kind: "normal",
    mirror: null,
    group: null,
  } as Parameter;
}

const curve: Curve = {
  target: { kind: "parameter", id: "AngleX" },
  segments: [
    { type: "start", p: { t: 0, v: 0 } },
    { type: "bezier", c1: { t: 0.3, v: 0 }, c2: { t: 0.7, v: 10 }, to: { t: 1, v: 10 } },
    { type: "stepped", to: { t: 1.5, v: -5 } },
    { type: "inverse_stepped", to: { t: 2, v: 5 } },
  ],
  fade_in: null,
  fade_out: null,
};

function motion(curves: Curve[] = [curve]): Motion {
  return { ...newMotion("Idle"), curves };
}

describe("time", () => {
  it("snaps to frames without float noise", () => {
    expect(snapToFrame(0.51)).toBe(0.5);
    expect(snapToFrame(1 / 30 + 0.001)).toBe(0.0333);
    expect(snapToFrame(-0.01)).toBe(-0);
  });

  it("advances, wraps when looping and stops at the end otherwise", () => {
    expect(advance(0.5, 0.25, 2, false)).toEqual({ time: 0.75, ended: false });
    expect(advance(1.9, 0.2, 2, false)).toEqual({ time: 2, ended: true });
    const looped = advance(1.9, 0.2, 2, true);
    expect(looped.ended).toBe(false);
    expect(looped.time).toBeCloseTo(0.1);
  });

  it("chooses ticks at least 60px apart", () => {
    expect(tickStep(2, 600)).toBe(0.25);
    expect(tickStep(10, 600)).toBe(1);
  });
});

describe("graph mapping", () => {
  const view = { t0: 0, t1: 2, v0: -10, v1: 10, width: 200, height: 100 };
  it("maps time and value both ways", () => {
    expect(timeToX(1, view)).toBe(100);
    expect(xToTime(50, view)).toBe(0.5);
    expect(valueToY(10, view)).toBe(0);
    expect(valueToY(-10, view)).toBe(100);
    expect(yToValue(50, view)).toBe(0);
  });

  it("draws beziers as SVG cubics and steps as corners", () => {
    const d = curvePath(curve, { t0: 0, t1: 2, v0: -10, v1: 10, width: 200, height: 100 });
    expect(d).toContain("C30,50 70,0 100,0");
    // stepped: 始点の値で終点の時刻まで行ってから下りる。
    expect(d).toContain("L150,0 L150,75");
    // inverse_stepped: 始点の時刻で上ってから進む。
    expect(d).toContain("L150,25 L200,25");
    expect(d.startsWith("M0,50 L0,50")).toBe(true);
  });

  it("picks the nearest mark whose hit square holds the point", () => {
    const marks = [
      { id: "a", x: 0, y: 0 },
      { id: "b", x: 22, y: 0 },
      { id: "c", x: 11, y: 20 },
    ];
    // a と b の押せる範囲 (24px) が重なる所では、中心の近い方
    expect(nearestMark(marks, { x: 10.5, y: 0 }, 24)?.id).toBe("a");
    expect(nearestMark(marks, { x: 11.5, y: 0 }, 24)?.id).toBe("b");
    // 等しければ先の印
    expect(nearestMark(marks, { x: 11, y: 0 }, 24)?.id).toBe("a");
    expect(nearestMark(marks, { x: 11, y: 10 }, 24)?.id).toBe("c");
    expect(nearestMark(marks, { x: 40, y: 0 }, 24)).toBeNull();
    // 距離は縦横の大きい方で測る (正方形の押せる範囲に合わせる): 斜めの 9px は、横の 11px より近い
    const diagonal = [
      { id: "side", x: 11, y: 0 },
      { id: "corner", x: 9, y: 9 },
    ];
    expect(nearestMark(diagonal, { x: 0, y: 0 }, 24)?.id).toBe("corner");
  });

  it("covers the parameter range and the keys", () => {
    expect(valueRange(curve, param("AngleX"))).toEqual([-30, 30]);
    expect(valueRange(curve, param("AngleX", 0, 0, 1))).toEqual([-5, 10]);
    expect(valueRange(null, null)).toEqual([0, 1]);
  });
});

describe("curves", () => {
  it("lists keys and segments", () => {
    expect(curveKeys(curve).map((k) => k.t)).toEqual([0, 1, 1.5, 2]);
    expect(segmentEndingAt(curve, 1)?.type).toBe("bezier");
    expect(segmentEndingAt(curve, 0)).toBeNull();
  });

  it("shapes each key after the segment into it", () => {
    expect(shapedKeys(curve).map((k) => [k.t, k.shape])).toEqual([
      [0, "diamond"],
      [1, "circle"],
      [1.5, "square"],
      [2, "square"],
    ]);
    const linear: Curve = {
      ...curve,
      segments: [
        { type: "start", p: { t: 0, v: 0 } },
        { type: "linear", to: { t: 1, v: 1 } },
      ],
    };
    expect(shapedKeys(linear).map((k) => k.shape)).toEqual(["diamond", "diamond"]);
  });

  it("does not move keys past their neighbors", () => {
    expect(canMoveKey(curve, 1, 1.2)).toBe(true);
    expect(canMoveKey(curve, 1, 1.5)).toBe(false);
    expect(canMoveKey(curve, 1, 0)).toBe(false);
    expect(canMoveKey(curve, 2, 5)).toBe(true);
    expect(canMoveKey(curve, 0.5, 0.6)).toBe(false);
  });

  it("follows the key times moved by pending commits", () => {
    expect(followKey(curve, 1, new Map([[1, 0.5]]))?.t).toBe(1);
    expect(followKey(curve, 0.8, new Map([[0.8, 1.5]]))?.t).toBe(1.5);
    expect(
      followKey(
        curve,
        0.7,
        new Map([
          [0.7, 0.8],
          [0.8, 2],
        ]),
      )?.t,
    ).toBe(2);
    expect(followKey(curve, 0.7, new Map())).toBeNull();
    expect(
      followKey(
        curve,
        0.7,
        new Map([
          [0.7, 0.8],
          [0.8, 0.7],
        ]),
      ),
    ).toBeNull();
  });

  it("remembers moved key times per motion and track until the sends finish", async () => {
    const moves = new KeyMoves();
    const other: Curve = { ...curve, target: { kind: "parameter", id: "AngleY" } };
    let finish = () => {};
    const sent = moves.track(
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
    );
    moves.record("Nod", curve, 0.8, 1);
    expect(moves.follow("Nod", curve, 0.8)?.t).toBe(1);
    expect(moves.follow("Shake", curve, 0.8)).toBeNull();
    expect(moves.follow("Nod", other, 0.8)).toBeNull();
    finish();
    await sent;
    expect(moves.follow("Nod", curve, 0.8)).toBeNull();
  });

  it("keeps handles inside their segment", () => {
    expect(clampHandle({ t: 3, v: 1 }, { t: 0, v: 0 }, { t: 1, v: 1 })).toEqual({ t: 1, v: 1 });
    expect(segmentShape("bezier")).toEqual({ type: "bezier" });
    expect(segmentShape("stepped")).toEqual({ type: "stepped" });
  });

  it("reads the handles of the bezier segment ending at a key", () => {
    expect(segmentHandle(curve, 1, "c1")).toEqual({ t: 0.3, v: 0 });
    expect(segmentHandle(curve, 1, "c2")).toEqual({ t: 0.7, v: 10 });
    expect(segmentHandle(curve, 1.5, "c1")).toBeNull();
    expect(segmentHandle(curve, 0, "c1")).toBeNull();
  });
});

describe("tracks", () => {
  it("lists parameters, then other curves", () => {
    const rows = tracks(
      motion([
        curve,
        {
          target: { kind: "part_opacity", id: "Hat" },
          segments: [{ type: "start", p: { t: 0, v: 1 } }],
          fade_in: null,
          fade_out: null,
        },
        {
          target: { kind: "parameter", id: "Gone" },
          segments: [{ type: "start", p: { t: 0, v: 1 } }],
          fade_in: null,
          fade_out: null,
        },
      ]),
      [param("AngleX"), param("AngleY")],
    );
    expect(rows.map((r) => r.label)).toEqual([
      "AngleX",
      "AngleY",
      "Opacity: Hat",
      "Gone (missing)",
    ]);
    expect(rows[0]?.curve).toBe(curve);
    expect(rows[1]?.curve).toBeNull();
  });
});

describe("keying from sliders", () => {
  const params = [param("AngleX"), param("AngleY"), param("AngleZ")];
  it("keys only changed parameters, in one batch", () => {
    const cmd = keyValuesCommand(motion(), params, { AngleX: 5, AngleY: 12, AngleZ: 0 }, 0.5, {
      AngleX: 5,
    });
    // AngleX はカーブの値と同じ、AngleZ は既定値のまま。
    expect(cmd).toEqual({
      op: "set_motion_key",
      motion: "Idle",
      target: { kind: "parameter", id: "AngleY" },
      point: { t: 0.5, v: 12 },
    });
    const both = keyValuesCommand(motion(), params, { AngleX: 6, AngleY: 12 }, 0.5, { AngleX: 5 });
    expect(both?.op).toBe("batch");
    expect(keyValuesCommand(motion(), params, { AngleX: 5 }, 0.5, { AngleX: 5 })).toBeNull();
  });

  it("keys the given parameters regardless of change", () => {
    const cmd = keyValuesCommand(motion([]), params, { AngleX: 0 }, 0, {}, ["AngleX"]);
    expect(cmd?.op).toBe("set_motion_key");
  });
});

describe("names", () => {
  it("accepts file-name characters and avoids duplicates", () => {
    expect(isValidName("Idle_01.v2")).toBe(true);
    expect(isValidName(".hidden")).toBe(false);
    expect(isValidName("笑顔")).toBe(false);
    expect(isValidName("a".repeat(129))).toBe(false);
    expect(uniqueName("Motion", ["motion", "Motion_2"])).toBe("Motion_3");
  });
});

describe("remembered panel state", () => {
  it("saves and loads whether the panel is open under the same key and values as before", () => {
    const storage = new MemoryStorage();
    useLocalStorage(storage);
    expect(loadPanelOpen()).toBe(true);
    savePanelOpen(false);
    expect(storage.getItem("ugoe.editor.timelineOpen")).toBe("closed");
    expect(loadPanelOpen()).toBe(false);
    savePanelOpen(true);
    expect(storage.getItem("ugoe.editor.timelineOpen")).toBe("open");
    expect(loadPanelOpen()).toBe(true);
    // 知らない値は開く
    storage.setItem("ugoe.editor.timelineOpen", "?");
    expect(loadPanelOpen()).toBe(true);
  });

  it("stays open when localStorage cannot be used", () => {
    for (const storage of [new BrokenStorage(), undefined]) {
      useLocalStorage(storage);
      expect(loadPanelOpen()).toBe(true);
      expect(() => savePanelOpen(false)).not.toThrow();
    }
  });
});
