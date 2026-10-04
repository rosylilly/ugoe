// 問い合わせの JSON (`*_json()`) の出力のキーと、`./queries.ts` の型のキーの突き合わせ (T0463)。
//
// 型は実行時に残らないので、型ごとのキーの一覧 (`QUERY_KEYS`) を値で持つ。一覧と型が同じことは `query-keys.test.ts` が
// `expectTypeOf` で確かめ (`pnpm check` の tsc)、本物の wasm の出力を一覧と比べるのは適合テストの `queries`
// (tests/conformance/web/queries.js)。型にキーを足したら一覧にも足す (足さないと `pnpm check` が落ちる)。
//
// パッケージの入口からは出さない (dist/query-keys.js はできるが、package.json の `exports` に無い)。

import {
  avatarSlots,
  currentPose,
  drawableStates,
  expressionInfos,
  moduleInfos,
  motionInfos,
  parameterDefinitions,
  slotDefinitions,
} from "./queries.js";
import type { WebPuppet } from "./wasm/ugoe_wasm.js";

/** 1 つの形のキー。`nested` はキーの値の形 (`list` なら配列の要素の形)。 */
export interface ShapeKeys {
  required: readonly string[];
  optional: readonly string[];
  nested?: Readonly<Record<string, { shape: string; list?: boolean }>>;
}

/** `./queries.ts` の型ごとのキー (`query-keys.test.ts` が型と同じことを確かめる)。 */
export const QUERY_KEYS = {
  /** `UgoeParameterDefinitions` (`parameters_json()`) */
  parameter_definitions: {
    required: ["parameters", "groups"],
    optional: [],
    nested: {
      parameters: { shape: "parameter", list: true },
      groups: { shape: "parameter_group", list: true },
    },
  },
  /** `UgoeParameterDefinition` */
  parameter: {
    required: ["id", "name", "min", "max", "default"],
    optional: ["internal", "kind", "mirror", "group"],
  },
  /** `UgoeParameterGroup` */
  parameter_group: { required: ["id", "name"], optional: [] },
  /** `UgoeMotionInfo` (`motions_json()` の要素) */
  motion_info: {
    required: ["name", "duration", "looping", "fade_in", "fade_out", "parameters", "events"],
    optional: [],
  },
  /** `UgoeExpressionInfo` (`expressions_json()` の要素) */
  expression_info: {
    required: ["name", "fade_in", "fade_out", "parameters"],
    optional: [],
    nested: { parameters: { shape: "expression_parameter", list: true } },
  },
  /** `UgoeExpressionParameter` */
  expression_parameter: { required: ["parameter", "value", "op"], optional: [] },
  /** `UgoeModuleInfo` (`modules_json()` の要素。`warning_details` の形は codes.test.ts) */
  module_info: {
    required: ["name", "slot", "requires", "attached", "textures", "warnings", "warning_details"],
    optional: [],
    nested: { requires: { shape: "module_requirements" } },
  },
  /** `UgoeModuleRequirements` */
  module_requirements: { required: [], optional: ["avatar_level", "slots", "parameters"] },
  /** `UgoeSlot` (`slots_json()` の要素) */
  slot: {
    required: ["id", "draw_order"],
    optional: [
      "anchor",
      "replaces",
      "parameters",
      "mask_sources",
      "part",
      "colliders",
      "pose_parts",
    ],
    nested: { draw_order: { shape: "slot_draw_order" } },
  },
  /** `UgoeSlot["draw_order"]` */
  slot_draw_order: { required: ["min", "max"], optional: [] },
  /** `UgoeDrawableState` (`drawable_states_json()` の要素) */
  drawable_state: {
    required: [
      "opacity",
      "draw_order",
      "multiply_color",
      "screen_color",
      "mask_strength",
      "visible",
    ],
    optional: [],
  },
  /** `UgoePose` (`pose_json()`) */
  pose: {
    required: ["fade_duration", "groups"],
    optional: [],
    nested: { groups: { shape: "pose_group", list: true } },
  },
  /** `UgoePose["groups"]` の要素 */
  pose_group: {
    required: ["entries"],
    optional: [],
    nested: { entries: { shape: "pose_entry", list: true } },
  },
  /** `UgoePoseEntry` */
  pose_entry: { required: ["part", "links"], optional: [] },
  /** `UgoeCurveValue` (`sample_motion_json`・`MotionSampler.sample` の要素) */
  curve_value: {
    required: ["target", "value"],
    optional: [],
    nested: { target: { shape: "curve_target" } },
  },
  /** `UgoeCurveTarget` */
  curve_target: { required: ["kind", "id"], optional: [] },
} as const satisfies Record<string, ShapeKeys>;

/** `QUERY_KEYS` の形の名前。 */
export type QueryShape = keyof typeof QUERY_KEYS;

/**
 * 出力のキーを `QUERY_KEYS` と比べ、食い違い (無い必須のキー・一覧に無いキー) を `errors` に、
 * 形ごとに比べた数を `seen` に貯める (見本のモデルがすべての形を出したかを確かめるため)。
 */
export class QueryKeyCheck {
  readonly errors: string[] = [];
  readonly seen: Partial<Record<QueryShape, number>> = {};

  /** `value` (JSON を読んだ値) を形 `shape` として比べる。`path` は食い違いの報告に使う。 */
  check(value: unknown, shape: QueryShape, path: string): void {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
      this.errors.push(`${path}: expected an object (${shape}), got ${JSON.stringify(value)}`);
      return;
    }
    this.seen[shape] = (this.seen[shape] ?? 0) + 1;
    const keys: ShapeKeys = QUERY_KEYS[shape];
    const known = new Set([...keys.required, ...keys.optional]);
    const record = value as Record<string, unknown>;
    for (const key of keys.required) {
      if (!(key in record)) this.errors.push(`${path}: missing key \`${key}\` (${shape})`);
    }
    for (const key of Object.keys(record)) {
      if (!known.has(key)) this.errors.push(`${path}: unknown key \`${key}\` (${shape})`);
    }
    for (const [key, nested] of Object.entries(keys.nested ?? {})) {
      const child = record[key];
      if (child === undefined) continue;
      const childShape = nested.shape as QueryShape;
      if (nested.list) this.list(child, childShape, `${path}.${key}`);
      else this.check(child, childShape, `${path}.${key}`);
    }
  }

  /** 配列の各要素を形 `shape` として比べる。 */
  list(value: unknown, shape: QueryShape, path: string): void {
    if (!Array.isArray(value)) {
      this.errors.push(`${path}: expected an array of ${shape}, got ${JSON.stringify(value)}`);
      return;
    }
    value.forEach((item, i) => {
      this.check(item, shape, `${path}[${i}]`);
    });
  }

  /** `puppet` の `*_json()` (警告・物理を除く。bindings/web/README.md#問い合わせの-json-の型-t0452) をすべて比べる。 */
  puppet(puppet: WebPuppet, name: string): void {
    this.check(parameterDefinitions(puppet), "parameter_definitions", `${name}.parameters_json()`);
    this.list(motionInfos(puppet), "motion_info", `${name}.motions_json()`);
    this.list(expressionInfos(puppet), "expression_info", `${name}.expressions_json()`);
    this.list(moduleInfos(puppet), "module_info", `${name}.modules_json()`);
    this.list(slotDefinitions(puppet), "slot", `${name}.slots_json()`);
    this.list(drawableStates(puppet), "drawable_state", `${name}.drawable_states_json()`);
    const pose = currentPose(puppet);
    if (pose !== undefined) this.check(pose, "pose", `${name}.pose_json()`);
    const slots = avatarSlots(puppet);
    if (slots !== null && !(Array.isArray(slots) && slots.every((s) => typeof s === "string"))) {
      this.errors.push(
        `${name}.avatar_slots_json(): expected string[] or null, got ${JSON.stringify(slots)}`,
      );
    }
  }

  /** 比べていない形 (見本が出さなかった形)。 */
  unseen(): QueryShape[] {
    return (Object.keys(QUERY_KEYS) as QueryShape[]).filter((s) => !this.seen[s]);
  }
}
