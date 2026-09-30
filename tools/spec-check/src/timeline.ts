// 時間の例 (spec/examples/README.md#timeline-examples) の読み込みと、期待値との比較。

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readModelJson } from "./bundle.ts";
import * as D from "./decode.ts";
import { compareMeshes, type ExpectedMesh } from "./examples.ts";
import { type FrameResult, Runtime, type Stage } from "./frame.ts";
import type { Priority } from "./motion.ts";
import type * as M from "./types.ts";
import { validatePhysics } from "./validate.ts";

export type Action =
  | { set: Record<string, number> }
  | { start_motion: string; priority: Priority; accepted?: boolean }
  | { stop_motions: true }
  | { set_expression: string }
  | { add_expression: string }
  | { stop_expressions: true }
  | { lip_sync: number | null }
  | { reset_physics: true }
  | { stages: Partial<Record<Stage, boolean>> };

export interface ExpectedFrame {
  parameters?: Record<string, number>;
  part_opacity?: Record<string, { value: number; weight: number }>;
  events?: string[];
  offsets?: {
    deformer_angles?: Record<string, number>;
    deformer_origins?: Record<string, M.Vec2>;
    warp_points?: Record<string, M.Vec2[]>;
    mesh_vertices?: Record<string, M.Vec2[]>;
  };
  meshes?: Record<string, ExpectedMesh>;
}

export interface Frame {
  actions?: Action[];
  dt: number;
  expect?: ExpectedFrame;
}

export interface Timeline {
  model: string;
  physics?: string;
  motions?: Record<string, string>;
  expressions?: Record<string, string>;
  summary: string;
  frames: Frame[];
}

export interface TimelineExample {
  timeline: Timeline;
  model: M.Model;
  physics: M.PhysicsSettings | null;
  motions: Map<string, M.Motion>;
  expressions: Map<string, M.Expression>;
  warnings: string[];
}

function readDoc<T>(
  path: string,
  conv: (c: D.Ctx, v: ReturnType<typeof D.parseJson>) => T,
  warnings: string[],
): T {
  return conv(new D.Ctx(warnings), D.parseJson(readFileSync(path, "utf8")));
}

/** 時間の例のディレクトリを読む。 */
export function loadTimeline(dir: string): TimelineExample {
  const timeline = JSON.parse(readFileSync(join(dir, "timeline.json"), "utf8")) as Timeline;
  const { model, warnings } = readModelJson(readFileSync(join(dir, timeline.model), "utf8"));
  let physics: M.PhysicsSettings | null = null;
  if (timeline.physics) {
    physics = readDoc(join(dir, timeline.physics), D.physicsSettings, warnings);
    const errors = validatePhysics(model, physics);
    if (errors.length > 0) throw new Error(`invalid physics: ${errors.join("; ")}`);
  }
  const motions = new Map(
    Object.entries(timeline.motions ?? {}).map(([k, f]) => [
      k,
      readDoc(join(dir, f), D.motion, warnings),
    ]),
  );
  const expressions = new Map(
    Object.entries(timeline.expressions ?? {}).map(([k, f]) => [
      k,
      readDoc(join(dir, f), D.expression, warnings),
    ]),
  );
  return { timeline, model, physics, motions, expressions, warnings };
}

/** 許容誤差 (spec/examples/README.md#timeline-tolerances)。 */
export const TIMELINE_TOLERANCE = {
  motion_parameter: 1e-4,
  physics_parameter: 1e-3,
  opacity: 1e-4,
  angle: 0.05,
  position: 0.05,
};

/** 物理の parameter 出力の対象のパラメータ。 */
function physicsTargets(physics: M.PhysicsSettings | null): Set<string> {
  const out = new Set<string>();
  for (const rig of physics?.rigs ?? []) {
    for (const o of rig.outputs) if (o.target.type === "parameter") out.add(o.target.id);
  }
  return out;
}

/** 例をはじめから走らせ、フレームごとの食い違いの説明を返す (空なら一致)。 */
export function runTimeline(ex: TimelineExample): string[] {
  const failures: string[] = [];
  const rt = new Runtime(ex.model, ex.physics);
  const phys = physicsTargets(ex.physics);
  ex.timeline.frames.forEach((frame, fi) => {
    const where = `frame ${fi}`;
    for (const action of frame.actions ?? []) {
      if ("set" in action) {
        for (const [id, v] of Object.entries(action.set)) rt.set(id, v);
      } else if ("start_motion" in action) {
        const m = ex.motions.get(action.start_motion);
        if (!m) throw new Error(`${where}: unknown motion ${action.start_motion}`);
        const accepted = rt.startMotion(m, action.priority);
        if (action.accepted !== undefined && accepted !== action.accepted) {
          failures.push(`${where}: start_motion ${action.start_motion}: accepted ${accepted}`);
        }
      } else if ("stop_motions" in action) rt.motions.stopAll();
      else if ("stop_expressions" in action) rt.expressions.stopAll();
      else if ("set_expression" in action || "add_expression" in action) {
        const key = "set_expression" in action ? action.set_expression : action.add_expression;
        const e = ex.expressions.get(key);
        if (!e) throw new Error(`${where}: unknown expression ${key}`);
        if ("set_expression" in action) rt.expressions.set(e);
        else rt.expressions.add(e);
      } else if ("lip_sync" in action) rt.lipSyncLevel = action.lip_sync;
      else if ("reset_physics" in action) rt.physics?.reset();
      else if ("stages" in action) {
        for (const [s, on] of Object.entries(action.stages)) rt.setStage(s as Stage, on ?? true);
      }
    }
    const result = rt.frame(frame.dt);
    if (frame.expect)
      failures.push(...compareFrame(ex, result, frame.expect, phys).map((f) => `${where}: ${f}`));
  });
  return failures;
}

function compareFrame(
  ex: TimelineExample,
  r: FrameResult,
  e: ExpectedFrame,
  phys: Set<string>,
): string[] {
  const failures: string[] = [];
  const near = (what: string, actual: number, expected: number, tol: number) => {
    if (!(Math.abs(actual - expected) <= tol))
      failures.push(`${what}: expected ${expected}, got ${actual}`);
  };
  const T = TIMELINE_TOLERANCE;
  for (const [id, v] of Object.entries(e.parameters ?? {})) {
    const p = ex.model.parameters.find((q) => q.id === id);
    if (!p) {
      failures.push(`parameter ${id} not found`);
      continue;
    }
    const range = p.max - p.min;
    const tol = phys.has(id)
      ? T.physics_parameter * range
      : T.motion_parameter * Math.max(1, range);
    near(`parameter ${id}`, r.values.get(id) ?? Number.NaN, v, tol);
  }
  for (const [id, o] of Object.entries(e.part_opacity ?? {})) {
    const a = r.overrides.get(id) ?? { value: 0, weight: 0 };
    near(`part_opacity ${id}.weight`, a.weight, o.weight, T.opacity);
    if (o.weight > 0) near(`part_opacity ${id}.value`, a.value, o.value, T.opacity);
  }
  if (e.events && JSON.stringify(e.events) !== JSON.stringify(r.events)) {
    failures.push(`events: expected ${JSON.stringify(e.events)}, got ${JSON.stringify(r.events)}`);
  }
  const off = e.offsets ?? {};
  for (const [id, v] of Object.entries(off.deformer_angles ?? {})) {
    near(`offset angle ${id}`, r.offsets.angles.get(id) ?? 0, v, T.angle);
  }
  for (const [id, v] of Object.entries(off.deformer_origins ?? {})) {
    const a = r.offsets.origins.get(id) ?? [0, 0];
    near(`offset origin ${id}.x`, a[0], v[0], T.position);
    near(`offset origin ${id}.y`, a[1], v[1], T.position);
  }
  for (const [kind, map, expected] of [
    ["points", r.offsets.points, off.warp_points],
    ["vertices", r.offsets.vertices, off.mesh_vertices],
  ] as const) {
    for (const [id, vs] of Object.entries(expected ?? {})) {
      const a = map.get(id) ?? [];
      vs.forEach((v, i) => {
        const x = a[i] ?? [0, 0];
        near(`offset ${kind} ${id}[${i}].x`, x[0], v[0], T.position);
        near(`offset ${kind} ${id}[${i}].y`, x[1], v[1], T.position);
      });
    }
  }
  if (e.meshes) failures.push(...compareMeshes(r.evaluation, e.meshes, T.position));
  return failures;
}
