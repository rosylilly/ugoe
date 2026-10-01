import { describe, expect, it } from "vitest";
import {
  defaultIdleMotion,
  describeMotion,
  describeRejectedMotion,
  loadToggle,
  type MotionInfo,
  motionOverTrackingActive,
  motionTrackingEffect,
  parseExpressions,
  parseMotions,
  saveToggle,
} from "./animation";

const nod: MotionInfo = {
  name: "nod",
  duration: 1.2,
  looping: false,
  fadeIn: 0.2,
  fadeOut: 0.3,
  parameters: ["AngleY", "BodyAngleY"],
};

describe("parseMotions", () => {
  it("reads the list and fills missing values", () => {
    const json = JSON.stringify([
      {
        name: "idle",
        duration: 4,
        looping: true,
        fade_in: 0.5,
        fade_out: 0.5,
        parameters: ["AngleX", 3],
        events: [],
      },
      { name: "short" },
      { duration: 1 },
    ]);
    expect(parseMotions(json)).toEqual([
      {
        name: "idle",
        duration: 4,
        looping: true,
        fadeIn: 0.5,
        fadeOut: 0.5,
        parameters: ["AngleX"],
      },
      { name: "short", duration: 0, looping: false, fadeIn: 0, fadeOut: 0, parameters: [] },
    ]);
    expect(parseMotions("{}")).toEqual([]);
  });
});

describe("parseExpressions", () => {
  it("reads parameters and ops", () => {
    const json = JSON.stringify([
      {
        name: "smile",
        fade_in: 0.3,
        fade_out: 0.3,
        parameters: [{ parameter: "MouthForm", value: 1, op: "overwrite" }, { value: 1 }, null],
      },
    ]);
    expect(parseExpressions(json)).toEqual([
      {
        name: "smile",
        fadeIn: 0.3,
        fadeOut: 0.3,
        parameters: [{ id: "MouthForm", op: "overwrite" }],
      },
    ]);
  });
});

describe("describeMotion", () => {
  it("shows the length, loop and fades", () => {
    expect(describeMotion(nod)).toBe("1.2 s · フェード 0.2 / 0.3 s");
    expect(describeMotion({ ...nod, looping: true, duration: 4, fadeIn: 0, fadeOut: 0 })).toBe(
      "4 s ループ",
    );
  });
});

describe("motionTrackingEffect", () => {
  it("tells how much of the motion the tracking overrides", () => {
    expect(motionTrackingEffect(nod, new Set())).toEqual({ kind: "none", overlap: [] });
    expect(motionTrackingEffect(nod, new Set(["AngleY", "EyeLOpen"]))).toEqual({
      kind: "some",
      overlap: ["AngleY"],
    });
    expect(motionTrackingEffect(nod, new Set(["AngleY", "BodyAngleY"])).kind).toBe("all");
    // パラメータを動かさないモーション (パーツの不透明度だけなど) は重ならない。
    expect(motionTrackingEffect({ ...nod, parameters: [] }, new Set(["AngleY"])).kind).toBe("none");
  });
});

it("describes a rejected motion with the priority label", () => {
  expect(describeRejectedMotion("nod", "idle")).toBe(
    "「nod」は、再生中のモーションより優先度 (待機) が低いので始まりませんでした",
  );
});

describe("defaultIdleMotion", () => {
  it("prefers a motion named idle, then a looping one starting with idle", () => {
    const m = (name: string, looping = false): MotionInfo => ({ ...nod, name, looping });
    expect(defaultIdleMotion([m("nod"), m("Idle"), m("idle_2", true)])).toBe("Idle");
    expect(defaultIdleMotion([m("idle_a"), m("idle_b", true)])).toBe("idle_b");
    expect(defaultIdleMotion([m("idle_a")])).toBe("idle_a");
    expect(defaultIdleMotion([m("nod"), m("sidle")])).toBeNull();
    expect(defaultIdleMotion([])).toBeNull();
  });
});

describe("motionOverTrackingActive", () => {
  it("applies only to motions above the idle priority while tracking", () => {
    expect(motionOverTrackingActive(true, true, "nod", "normal")).toBe(true);
    expect(motionOverTrackingActive(true, true, "nod", "force")).toBe(true);
    expect(motionOverTrackingActive(true, true, "idle", "idle")).toBe(false);
    expect(motionOverTrackingActive(true, true, null, "normal")).toBe(false);
    expect(motionOverTrackingActive(true, false, "nod", "normal")).toBe(false);
    expect(motionOverTrackingActive(false, true, "nod", "normal")).toBe(false);
  });
});

describe("loadToggle / saveToggle", () => {
  function memoryStorage(): Storage {
    const map = new Map<string, string>();
    return {
      getItem: (k) => map.get(k) ?? null,
      setItem: (k, v) => void map.set(k, v),
      removeItem: (k) => void map.delete(k),
      clear: () => map.clear(),
      key: () => null,
      get length() {
        return map.size;
      },
    };
  }

  it("remembers on and off and falls back otherwise", () => {
    const storage = memoryStorage();
    expect(loadToggle("k", true, storage)).toBe(true);
    saveToggle("k", false, storage);
    expect(loadToggle("k", true, storage)).toBe(false);
    saveToggle("k", true, storage);
    expect(loadToggle("k", false, storage)).toBe(true);
    storage.setItem("k", "maybe");
    expect(loadToggle("k", false, storage)).toBe(false);
    const broken = {
      getItem: () => {
        throw new Error("denied");
      },
      setItem: () => {
        throw new Error("denied");
      },
    } as unknown as Storage;
    expect(loadToggle("k", true, broken)).toBe(true);
    expect(() => saveToggle("k", true, broken)).not.toThrow();
  });
});
