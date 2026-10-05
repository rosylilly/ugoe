// 問い合わせの JSON のキーの一覧 (`QUERY_KEYS`) と `./queries.ts` の型の突き合わせ (T0463)。
//
// 一覧が型と同じことを `expectTypeOf` で確かめる (`pnpm check` の tsc が見る)。本物の wasm の出力と一覧を比べるのは
// 適合テストの `queries` (tests/conformance/web/queries.js)。ここでは偽の `WebPuppet` で食い違いを見つけることを確かめる。

import { describe, expect, expectTypeOf, it, vi } from "vitest";
import type {
  UgoeCurveTarget,
  UgoeCurveValue,
  UgoeDrawableState,
  UgoeExpressionInfo,
  UgoeExpressionParameter,
  UgoeModuleInfo,
  UgoeModuleRequirements,
  UgoeMotionEvent,
  UgoeMotionInfo,
  UgoeParameterDefinition,
  UgoeParameterDefinitions,
  UgoeParameterGroup,
  UgoePose,
  UgoePoseEntry,
  UgoeSlot,
} from "./queries.js";
import { QUERY_KEYS, QueryKeyCheck, type QueryShape } from "./query-keys.js";
import type { WebPuppet } from "./wasm/ugoe_wasm.js";

vi.mock("./wasm/ugoe_wasm.js", () => ({ default: vi.fn(), WebPuppet: class {} }));

/** 型 `T` の名前の付いたキー (`[key: string]: unknown` を除く)。 */
type KnownKeys<T> = keyof {
  [K in keyof T as string extends K ? never : number extends K ? never : K]: T[K];
};
/** 型 `T` の省略できないキー。 */
type RequiredKeys<T> = {
  [K in KnownKeys<T>]-?: object extends Pick<T, K> ? never : K;
}[KnownKeys<T>];
/** 型 `T` の省略できるキー。 */
type OptionalKeys<T> = Exclude<KnownKeys<T>, RequiredKeys<T>>;

/** 一覧の形 → `./queries.ts` の型。 */
type TypeOf = {
  parameter_definitions: UgoeParameterDefinitions;
  parameter: UgoeParameterDefinition;
  parameter_group: UgoeParameterGroup;
  motion_info: UgoeMotionInfo;
  motion_event: UgoeMotionEvent;
  expression_info: UgoeExpressionInfo;
  expression_parameter: UgoeExpressionParameter;
  module_info: UgoeModuleInfo;
  module_requirements: UgoeModuleRequirements;
  slot: UgoeSlot;
  slot_draw_order: UgoeSlot["draw_order"];
  drawable_state: UgoeDrawableState;
  pose: UgoePose;
  pose_group: UgoePose["groups"][number];
  pose_entry: UgoePoseEntry;
  curve_value: UgoeCurveValue;
  curve_target: UgoeCurveTarget;
};

describe("QUERY_KEYS and the types in queries.ts", () => {
  it("lists the same keys as each type", () => {
    expectTypeOf<keyof TypeOf>().toEqualTypeOf<QueryShape>();
    expectTypeOf<{
      [S in QueryShape]: (typeof QUERY_KEYS)[S]["required"][number];
    }>().toEqualTypeOf<{ [S in QueryShape]: RequiredKeys<TypeOf[S]> }>();
    expectTypeOf<{
      [S in QueryShape]: (typeof QUERY_KEYS)[S]["optional"][number];
    }>().toEqualTypeOf<{ [S in QueryShape]: OptionalKeys<TypeOf[S]> }>();
  });

  it("names every nested shape", () => {
    for (const keys of Object.values(QUERY_KEYS)) {
      for (const [key, nested] of Object.entries("nested" in keys ? keys.nested : {})) {
        expect(Object.keys(QUERY_KEYS)).toContain(nested.shape);
        expect([...keys.required, ...keys.optional]).toContain(key);
      }
    }
  });
});

/** `WebPuppet` の `*_json()` だけを持つ偽物 (メソッドの名前 → 返す値。`undefined` はそのまま返す)。 */
const puppetWith = (values: Record<string, unknown>) =>
  Object.fromEntries(
    Object.entries(values).map(([name, value]) => [
      name,
      () => (value === undefined ? undefined : JSON.stringify(value)),
    ]),
  ) as unknown as WebPuppet;

/** すべての形を 1 つずつ出す偽物の出力。 */
const outputs = () => ({
  parameters_json: {
    parameters: [{ id: "AngleX", name: "Angle X", min: -30, max: 30, default: 0, kind: "normal" }],
    groups: [{ id: "Face", name: "Face" }],
  },
  motions_json: [
    {
      name: "idle",
      duration: 2,
      looping: true,
      fade_in: 0.5,
      fade_out: 0.5,
      parameters: [],
      events: [],
    },
  ] as Record<string, unknown>[],
  motion_events_json: [{ value: "wave", time: 0.5, motion: "idle", number: 1 }],
  expressions_json: [
    {
      name: "smile",
      fade_in: 0.2,
      fade_out: 0.2,
      parameters: [{ parameter: "MouthForm", value: 1, op: "overwrite" }],
    },
  ],
  modules_json: [
    {
      name: "Hat",
      slot: "Head",
      requires: { avatar_level: 1 },
      attached: true,
      textures: 1,
      warnings: [],
      warning_details: [],
    },
  ],
  slots_json: [{ id: "Head", draw_order: { min: 0, max: 10 } }],
  avatar_slots_json: null as unknown,
  drawable_states_json: [
    {
      opacity: 1,
      draw_order: 500,
      multiply_color: [1, 1, 1],
      screen_color: [0, 0, 0],
      mask_strength: 1,
      visible: true,
    },
  ],
  pose_json: { fade_duration: 0.5, groups: [{ entries: [{ part: "ArmA", links: [] }] }] },
});

describe("QueryKeyCheck", () => {
  it("accepts outputs with the listed keys and counts the shapes", () => {
    const check = new QueryKeyCheck();
    check.puppet(puppetWith(outputs()), "fake");
    check.list([{ target: { kind: "parameter", id: "AngleX" }, value: 1 }], "curve_value", "s");
    expect(check.errors).toEqual([]);
    expect(check.unseen()).toEqual([]);
    expect(check.seen.parameter).toBe(1);
  });

  it("reports unknown and missing keys with their path", () => {
    const values = outputs();
    values.motions_json[0] = { ...values.motions_json[0], speed: 1 };
    delete values.motions_json[0]?.events;
    values.pose_json.groups[0]?.entries.push({ part: "ArmB" } as never);
    values.avatar_slots_json = [1];
    const check = new QueryKeyCheck();
    check.puppet(puppetWith(values), "fake");
    expect(check.errors).toEqual([
      "fake.motions_json()[0]: missing key `events` (motion_info)",
      "fake.motions_json()[0]: unknown key `speed` (motion_info)",
      "fake.pose_json().groups[0].entries[1]: missing key `links` (pose_entry)",
      "fake.avatar_slots_json(): expected string[] or null, got [1]",
    ]);
  });

  it("reports shapes that were not seen", () => {
    const check = new QueryKeyCheck();
    check.puppet(
      puppetWith({ ...outputs(), modules_json: [], motion_events_json: [], pose_json: undefined }),
      "without modules",
    );
    expect(check.unseen()).toEqual([
      "motion_event",
      "module_info",
      "module_requirements",
      "pose",
      "pose_group",
      "pose_entry",
      "curve_value",
      "curve_target",
    ]);
  });
});
