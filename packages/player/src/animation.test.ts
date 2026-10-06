import { setLocale } from "@ugoe/i18n";
import { beforeEach, describe, expect, it } from "vitest";
import {
  defaultIdleMotion,
  describeIdleChoice,
  describeMotion,
  describeRejectedMotion,
  groupTrackingParameters,
  idleMemory,
  initialIdleMotion,
  loadTrackingKept,
  type MotionInfo,
  motionOverTrackingActive,
  motionTrackingEffect,
  parseExpressions,
  parseIdleChoice,
  parseMotionEvents,
  parseMotions,
  saveTrackingKept,
  trackingGroup,
  trackingWins,
} from "./animation";
import { PlayerStorage } from "./storage";

// 文言は日本語の訳で確かめる (英語の原文を確かめる it は、その中で言語を切り替える)。
beforeEach(() => setLocale("ja"));

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

describe("parseMotionEvents", () => {
  it("reads the value, the time and the motion that fired each event", () => {
    const json = JSON.stringify([
      { value: "blink", time: 0.5, motion: "idle", number: 1 },
      { value: "wave", time: 0.25, motion: "wave", number: 2 },
    ]);
    expect(parseMotionEvents(json)).toEqual([
      { value: "blink", time: 0.5, motion: "idle", number: 1 },
      { value: "wave", time: 0.25, motion: "wave", number: 2 },
    ]);
  });

  it("skips entries without a value or a motion and zeroes missing numbers", () => {
    const json = JSON.stringify([
      { value: "a", motion: "m" },
      { value: 1, motion: "m", number: 1 },
      { value: "b", number: 1 },
      null,
      "c",
    ]);
    expect(parseMotionEvents(json)).toEqual([{ value: "a", time: 0, motion: "m", number: 0 }]);
    expect(parseMotionEvents("{}")).toEqual([]);
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
  setLocale("en");
  expect(describeRejectedMotion("nod", "idle")).toBe(
    '"nod" did not start because its priority (Idle) is lower than the playing motion.',
  );
  expect(describeMotion(nod)).toBe("1.2 s · fade 0.2 / 0.3 s");
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

describe("idle motion memory (T0722)", () => {
  const m = (name: string, looping = false): MotionInfo => ({ ...nod, name, looping });

  it("starts with the remembered choice when the model still has it", () => {
    const motions = [m("idle", true), m("tilt"), m("nod")];
    expect(initialIdleMotion(motions, null)).toBe("idle");
    expect(initialIdleMotion(motions, { motion: "tilt" })).toBe("tilt");
    // 「(なし)」を選んだことも覚える。
    expect(initialIdleMotion(motions, { motion: null })).toBeNull();
    // モデルから消えたモーションなら既定に戻る。
    expect(initialIdleMotion(motions, { motion: "gone" })).toBe("idle");
  });

  it("reads only the expected shape", () => {
    expect(parseIdleChoice({ motion: "idle" })).toEqual({ motion: "idle" });
    expect(parseIdleChoice({ motion: null })).toEqual({ motion: null });
    expect(parseIdleChoice({ motion: 3 })).toBeNull();
    expect(parseIdleChoice({})).toBeNull();
    expect(parseIdleChoice("idle")).toBeNull();
    expect(describeIdleChoice({ motion: "idle" })).toBe("idle");
    expect(describeIdleChoice({ motion: null })).toBe("(なし)");
  });

  it("remembers the choice per model in localStorage", () => {
    const items = new Map<string, string>();
    const storage = {
      getItem: (k: string) => items.get(k) ?? null,
      setItem: (k: string, v: string) => void items.set(k, v),
    } as unknown as Storage;
    const store = new PlayerStorage(undefined, storage);
    const memory = idleMemory(store);
    const a = { name: "a.ugoe", fingerprint: "fa" };
    memory.set(a, { motion: "tilt" });
    memory.set({ name: "b.ugoe", fingerprint: "fb" }, { motion: null });
    expect(idleMemory(store).get(a)).toEqual({ motion: "tilt" });
    expect(idleMemory(store).get({ name: "b.ugoe", fingerprint: "fb" })).toEqual({
      motion: null,
    });
    expect(items.has("ugoe.viewer.idle-motion.models")).toBe(true);
  });
});

describe("motion over tracking per parameter (T0722)", () => {
  it("groups the tracked parameters by the part of the face", () => {
    expect(trackingGroup("AngleX")).toBe("head");
    expect(trackingGroup("BodyAngleZ")).toBe("body");
    expect(trackingGroup("ShoulderLY")).toBe("body");
    expect(trackingGroup("EyeBallX")).toBe("eyes");
    expect(trackingGroup("BrowInnerUp")).toBe("brows");
    expect(trackingGroup("JawX")).toBe("mouth");
    expect(trackingGroup("CheekPuff")).toBe("mouth");
    expect(trackingGroup("HairSwing")).toBe("other");
    expect(
      groupTrackingParameters(["EyeLOpen", "AngleX", "MouthOpenY", "AngleY"]).map((g) => [
        g.group,
        g.ids,
      ]),
    ).toEqual([
      ["head", ["AngleX", "AngleY"]],
      ["eyes", ["EyeLOpen"]],
      ["mouth", ["MouthOpenY"]],
    ]);
  });

  it("lets only the kept parameters hide the motion while the motion wins", () => {
    const tracked = new Set(["AngleX", "EyeLOpen"]);
    expect([...trackingWins(tracked, false, ["EyeLOpen"])]).toEqual(["AngleX", "EyeLOpen"]);
    expect([...trackingWins(tracked, true, ["EyeLOpen", "Other"])]).toEqual(["EyeLOpen"]);
    expect([...trackingWins(tracked, true, [])]).toEqual([]);
  });

  it("remembers the kept parameters", () => {
    const items = new Map<string, string>();
    const storage = {
      getItem: (k: string) => items.get(k) ?? null,
      setItem: (k: string, v: string) => void items.set(k, v),
    } as unknown as Storage;
    const store = new PlayerStorage(undefined, storage);
    expect(loadTrackingKept(store)).toEqual([]);
    saveTrackingKept(["EyeLOpen", "EyeROpen"], store);
    expect(loadTrackingKept(store)).toEqual(["EyeLOpen", "EyeROpen"]);
    const key = "ugoe.viewer.motion-over-tracking.kept";
    items.set(key, '["A", 3, "A"]');
    expect(loadTrackingKept(store)).toEqual(["A"]);
    items.set(key, "{broken");
    expect(loadTrackingKept(store)).toEqual([]);
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
