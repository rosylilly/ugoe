// 問い合わせの JSON の型と読む補助 (T0452)。

import { describe, expect, expectTypeOf, it, vi } from "vitest";
import {
  avatarSlots,
  currentPose,
  drawableStates,
  expressionInfos,
  moduleInfos,
  motionEvents,
  motionInfos,
  parameterDefinitions,
  slotDefinitions,
  type UgoeCurveValue,
  type UgoeModuleInfo,
  visiblePoseParts,
} from "./queries.js";
import { sampleMotion } from "./runtime.js";
import { isKnownLoadWarning } from "./warnings.js";
import type { MotionSampler, WebPuppet } from "./wasm/ugoe_wasm.js";

vi.mock("./wasm/ugoe_wasm.js", () => ({
  default: vi.fn(),
  WebPuppet: class {},
  sample_motion_json: (json: string, t: number) =>
    JSON.stringify([{ target: { kind: "parameter", id: JSON.parse(json).name }, value: t }]),
}));

/** `WebPuppet` の `*_json()` だけを持つ偽物 (メソッドの名前 → 返す値。`undefined` はそのまま返す)。 */
const puppetWith = (values: Record<string, unknown>) =>
  Object.fromEntries(
    Object.entries(values).map(([name, value]) => [
      name,
      () => (value === undefined ? undefined : JSON.stringify(value)),
    ]),
  ) as unknown as WebPuppet;

describe("parameterDefinitions", () => {
  it("reads parameters and groups", () => {
    const json = {
      parameters: [
        {
          id: "AngleX",
          name: "Angle X",
          min: -30,
          max: 30,
          default: 0,
          kind: "normal",
          mirror: null,
          group: "Face",
        },
        { id: "Relay", name: "Relay", min: 0, max: 1, default: 0, internal: true },
      ],
      groups: [{ id: "Face", name: "Face" }],
    };
    const parsed = parameterDefinitions(puppetWith({ parameters_json: json }));
    expect(parsed).toEqual(json);
    expect(parsed.parameters.filter((p) => !p.internal).map((p) => p.id)).toEqual(["AngleX"]);
    expectTypeOf(parsed.groups[0]?.name).toEqualTypeOf<string | undefined>();
  });
});

describe("motionInfos and expressionInfos", () => {
  it("reads the motion and expression lists", () => {
    const motions = [
      {
        name: "idle",
        duration: 2,
        looping: true,
        fade_in: 0.5,
        fade_out: 0.5,
        parameters: ["AngleX"],
        events: ["half"],
      },
    ];
    const expressions = [
      {
        name: "smile",
        fade_in: 0.2,
        fade_out: 0.2,
        parameters: [{ parameter: "MouthForm", value: 1, op: "overwrite" }],
      },
    ];
    const puppet = puppetWith({ motions_json: motions, expressions_json: expressions });
    expect(motionInfos(puppet)).toEqual(motions);
    const parsed = expressionInfos(puppet);
    expect(parsed).toEqual(expressions);
    expectTypeOf(parsed[0]?.parameters[0]?.op).toEqualTypeOf<
      "add" | "multiply" | "overwrite" | undefined
    >();
  });
});

describe("motionEvents", () => {
  it("reads the events with the motion that fired them", () => {
    // 始め直した idle (2 番目) がフェードインする間、フェードアウト中の 1 番目もイベントを発火する。
    const events = [
      { value: "half", time: 0.5, motion: "idle", number: 1 },
      { value: "start", time: 0, motion: "idle", number: 2 },
    ];
    const parsed = motionEvents(puppetWith({ motion_events_json: events }));
    expect(parsed).toEqual(events);
    expect(parsed.map((e) => `${e.motion}#${e.number}`)).toEqual(["idle#1", "idle#2"]);
    expectTypeOf(parsed[0]?.number).toEqualTypeOf<number | undefined>();
    expect(motionEvents(puppetWith({ motion_events_json: [] }))).toEqual([]);
  });
});

describe("moduleInfos, slotDefinitions and avatarSlots", () => {
  it("reads modules with structured warnings", () => {
    const modules: UgoeModuleInfo[] = [
      {
        name: "Hat",
        slot: "Head",
        requires: { avatar_level: 1 },
        attached: true,
        textures: 1,
        warnings: ["texture 0 is KTX2"],
        warning_details: [{ code: "texture_ktx2", warning: "texture 0 is KTX2", texture: 0 }],
      },
    ];
    const parsed = moduleInfos(puppetWith({ modules_json: modules }));
    expect(parsed).toEqual(modules);
    const detail = parsed[0]?.warning_details[0];
    expect(detail && isKnownLoadWarning(detail) && detail.code === "texture_ktx2").toBe(true);
  });

  it("reads slots without the omitted keys", () => {
    const slots = [
      { id: "Hair", draw_order: { min: 100, max: 200 }, parameters: ["HairSwing"] },
      { id: "Hat", anchor: "Head", draw_order: { min: 300, max: 400 }, part: "HeadPart" },
    ];
    const parsed = slotDefinitions(puppetWith({ slots_json: slots }));
    expect(parsed).toEqual(slots);
    expect(parsed[0]?.anchor).toBeUndefined();
  });

  it("reads published slots or null", () => {
    expect(avatarSlots(puppetWith({ avatar_slots_json: ["Hair"] }))).toEqual(["Hair"]);
    expect(avatarSlots(puppetWith({ avatar_slots_json: null }))).toBeNull();
  });
});

describe("drawableStates and currentPose", () => {
  it("reads the evaluated drawables", () => {
    const states = [
      {
        opacity: 1,
        draw_order: 500,
        multiply_color: [1, 1, 1],
        screen_color: [0, 0, 0],
        mask_strength: 1,
        visible: true,
      },
    ];
    const parsed = drawableStates(puppetWith({ drawable_states_json: states }));
    expect(parsed).toEqual(states);
    expectTypeOf(parsed[0]?.multiply_color).toEqualTypeOf<[number, number, number] | undefined>();
  });

  it("reads the pose, or undefined without a model", () => {
    const pose = { fade_duration: 0.5, groups: [{ entries: [{ part: "ArmA", links: [] }] }] };
    expect(currentPose(puppetWith({ pose_json: pose }))).toEqual(pose);
    expect(currentPose(puppetWith({ pose_json: undefined }))).toBeUndefined();
  });

  it("reads the shown pose parts per group, or undefined without a model", () => {
    const parsed = visiblePoseParts(puppetWith({ pose_visible_json: [null, "ArmB"] }));
    expect(parsed).toEqual([null, "ArmB"]);
    expectTypeOf(parsed).toEqualTypeOf<(string | null)[] | undefined>();
    expect(visiblePoseParts(puppetWith({ pose_visible_json: undefined }))).toBeUndefined();
  });
});

describe("sampleMotion", () => {
  it("parses sample_motion_json", () => {
    const values = sampleMotion(JSON.stringify({ name: "AngleX" }), 0.25);
    expectTypeOf(values).toEqualTypeOf<UgoeCurveValue[]>();
    expect(values).toEqual([{ target: { kind: "parameter", id: "AngleX" }, value: 0.25 }]);
  });

  it("parses MotionSampler.sample", () => {
    const sampled: number[] = [];
    const sampler = {
      sample: (t: number) => {
        sampled.push(t);
        return JSON.stringify([{ target: { kind: "part_opacity", id: "ArmA" }, value: t / 2 }]);
      },
    } as unknown as MotionSampler;
    const values = sampleMotion(sampler, 0.5);
    expectTypeOf(values).toEqualTypeOf<UgoeCurveValue[]>();
    expect(values).toEqual([{ target: { kind: "part_opacity", id: "ArmA" }, value: 0.25 }]);
    expect(sampled).toEqual([0.5]);
  });
});
