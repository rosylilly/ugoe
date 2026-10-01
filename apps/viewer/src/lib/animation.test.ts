import { describe, expect, it } from "vitest";
import {
  activeExpressions,
  describeMotion,
  describeRejectedMotion,
  type MotionInfo,
  motionTrackingEffect,
  parseExpressions,
  parseMotions,
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

describe("activeExpressions", () => {
  it("follows set, add and stop", () => {
    let active = activeExpressions.set(["sad"], "smile");
    expect(active).toEqual(["smile"]);
    active = activeExpressions.add(active, "surprised");
    expect(active).toEqual(["smile", "surprised"]);
    expect(activeExpressions.add(active, "smile")).toEqual(["smile", "surprised"]);
    expect(activeExpressions.stop(active, "smile")).toEqual(["surprised"]);
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
