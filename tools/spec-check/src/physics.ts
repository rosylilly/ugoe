// 物理: 振り子の鎖、パラメータへの出力、リグの順と中継、動的な物理 (spec/physics.md)。
//
// 計算は倍精度で行う。手順と足し合わせの順は仕様の擬似コードのとおり。

import {
  apply,
  type DeformerState,
  emptyOffsets,
  jacobian,
  type Offsets,
  type ParameterValues,
} from "./evaluate.ts";
import type * as M from "./types.ts";

type Vec2 = M.Vec2;

const DEG = Math.PI / 180;
const clamp = (x: number, lo: number, hi: number): number => Math.min(Math.max(x, lo), hi);
const add = (a: Vec2, b: Vec2): Vec2 => [a[0] + b[0], a[1] + b[1]];
const sub = (a: Vec2, b: Vec2): Vec2 => [a[0] - b[0], a[1] - b[1]];
const mul = (a: Vec2, s: number): Vec2 => [a[0] * s, a[1] * s];
const len = (a: Vec2): number => Math.hypot(a[0], a[1]);
const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;

/** `R(θ) · v`。`θ` はラジアン、画面で時計回りが正 (spec/evaluation.md#overview)。 */
function rotRad(v: Vec2, theta: number): Vec2 {
  const c = Math.cos(theta);
  const s = Math.sin(theta);
  return [c * v[0] - s * v[1], s * v[0] + c * v[1]];
}

/** `rotate(v, θ)`。`θ` は度 (spec/physics.md#coordinates-and-units)。 */
const rotate = (v: Vec2, deg: number): Vec2 => rotRad(v, deg * DEG);

/** `normalize(v)`。長さが 0 か有限でなければ `d` (spec/physics.md#coordinates-and-units)。 */
function normalize(v: Vec2, d: Vec2): Vec2 {
  const l = len(v);
  if (!(l > 0) || !Number.isFinite(l)) return d;
  return [v[0] / l, v[1] / l];
}

/** `a` から `b` への角度 (ラジアン、時計回りが正)。どちらかが零ベクトルなら 0。 */
function signedAngleRad(a: Vec2, b: Vec2): number {
  if ((a[0] === 0 && a[1] === 0) || (b[0] === 0 && b[1] === 0)) return 0;
  return Math.atan2(a[0] * b[1] - a[1] * b[0], a[0] * b[0] + a[1] * b[1]);
}

/** `signed_angle(a, b)` (度)。 */
export const signedAngle = (a: Vec2, b: Vec2): number => signedAngleRad(a, b) / DEG;

/** `x` を (−180, 180] 度に写す。 */
export function wrap180(x: number): number {
  let r = x - 360 * Math.floor(x / 360);
  if (r > 180) r -= 360;
  return r;
}

/** `x` を (−π, π] に写す。 */
function wrapPi(x: number): number {
  let r = x - 2 * Math.PI * Math.floor(x / (2 * Math.PI));
  if (r > Math.PI) r -= 2 * Math.PI;
  return r;
}

/** 根の姿勢 (spec/physics.md#inputs)。`frame_angle` は動的な物理の重力の座標系の角度。 */
interface Pose {
  t: Vec2;
  angle: number;
  frame_angle: number;
}

/** 出力の値: パラメータと角度はスカラー、変位はベクトル、値が無ければ null。 */
type OutputValue = number | Vec2 | null;

interface RigState {
  position: Vec2[];
  last: Vec2[];
  velocity: Vec2[];
  pose: Pose;
  external: Pose;
  previous: OutputValue[];
  current: OutputValue[];
}

/** 当たり判定の、ワールドに置いた形 (spec/physics.md#colliders)。 */
type Core =
  | { kind: "circle"; c: Vec2; r: number }
  | { kind: "capsule"; a: Vec2; b: Vec2; r: number };

/** 1 回の更新の間に固定する、リグごとの動的な物理の文脈。 */
interface RigContext {
  anchor: Vec2;
  frameAngle: number;
  /** 出力ごとの `deformer_angle` の基準 (粒子 1 の親の向き) と、鏡映なら -1。 */
  angleRef: (Vec2 | null)[];
  angleSign: number[];
  colliders: Core[];
}

/** 解決した出力の対象。解決できないものは `ignored`。 */
type ResolvedTarget =
  | { kind: "parameter"; id: string }
  | { kind: "deformer_angle" | "deformer_offset"; id: string }
  | { kind: "points"; id: string; weights: number[] }
  | { kind: "vertices"; id: string; weights: number[] }
  | { kind: "ignored" };

export interface PhysicsOptions {
  /** `physics.dynamic` に対応するか (しないなら動的な物理のデータを無視する)。 */
  dynamic?: boolean;
}

/** 物理の実行状態 (spec/physics.md#state)。 */
export class Physics {
  readonly model: M.Model;
  readonly settings: M.PhysicsSettings;
  readonly dynamic: boolean;
  private accumulator = 0;
  private initialized = false;
  private rigs: RigState[] = [];
  private readonly targets: ResolvedTarget[][];
  private readonly relayInput: boolean[][];
  private readonly relayTargets: Set<string>;
  private readonly params: Map<string, M.Parameter>;
  /** 最後の更新で作ったオフセット。 */
  offsets: Offsets = emptyOffsets();

  constructor(model: M.Model, settings: M.PhysicsSettings, options: PhysicsOptions = {}) {
    this.model = model;
    this.settings = settings;
    this.dynamic = options.dynamic ?? true;
    this.params = new Map(model.parameters.map((p) => [p.id, p]));
    this.targets = settings.rigs.map((rig) => rig.outputs.map((o) => this.resolve(o)));
    // 中継の入力: 配列の前のリグの parameter 出力の対象を読む入力 (spec/physics.md#rig-order-and-relays)
    this.relayTargets = new Set();
    this.relayInput = settings.rigs.map((rig, r) => {
      const earlier = new Set<string>();
      for (let q = 0; q < r; q++) {
        for (const t of this.targets[q]) if (t.kind === "parameter") earlier.add(t.id);
      }
      return rig.inputs.map((i) => earlier.has(i.parameter));
    });
    for (const ts of this.targets) {
      for (const t of ts) if (t.kind === "parameter") this.relayTargets.add(t.id);
    }
  }

  /** 設定が動的な物理を使うか (spec/physics.md#dynamic-physics)。 */
  get usesDynamic(): boolean {
    return this.dynamic && this.settings.rigs.some((r) => this.rigIsDynamic(r));
  }

  private rigIsDynamic(rig: M.PhysicsRig): boolean {
    return (
      rig.frame !== null ||
      rig.colliders.length > 0 ||
      rig.outputs.some((o) => o.target.type !== "parameter" && o.target.type !== "unknown")
    );
  }

  /** リセット (spec/physics.md#state)。 */
  reset(): void {
    this.accumulator = 0;
    this.initialized = false;
    this.rigs = [];
    this.offsets = emptyOffsets();
  }

  private resolve(o: M.PhysicsOutput): ResolvedTarget {
    const t = o.target;
    const deformer = (id: string) => this.model.deformers.find((d) => d.id === id);
    switch (t.type) {
      case "parameter":
        return this.model.parameters.some((p) => p.id === t.id)
          ? { kind: "parameter", id: t.id }
          : { kind: "ignored" };
      case "deformer_angle":
      case "deformer_offset": {
        if (!this.dynamic) return { kind: "ignored" };
        const d = deformer(t.deformer);
        return d?.kind.type === "rotation" ? { kind: t.type, id: t.deformer } : { kind: "ignored" };
      }
      case "vertices": {
        if (!this.dynamic) return { kind: "ignored" };
        if (t.owner.kind === "warp") {
          const d = deformer(t.owner.id);
          return d?.kind.type === "warp"
            ? { kind: "points", id: t.owner.id, weights: t.weights }
            : { kind: "ignored" };
        }
        return this.model.meshes.some((m) => m.id === t.owner.id)
          ? { kind: "vertices", id: t.owner.id, weights: t.weights }
          : { kind: "ignored" };
      }
      case "unknown":
        return { kind: "ignored" };
    }
  }

  // ---- 入力 (spec/physics.md#inputs) ----

  private rootPose(r: number, values: ParameterValues, relay: boolean): Pose {
    const rig = this.settings.rigs[r];
    const pose: Pose = { t: [0, 0], angle: 0, frame_angle: 0 };
    rig.inputs.forEach((input, k) => {
      if (this.relayInput[r][k] !== relay) return;
      const p = this.params.get(input.parameter);
      if (!p || input.source === "unknown") return;
      const range = input.source === "angle" ? rig.normalization.angle : rig.normalization.position;
      let n = normalizeInput(values.get(p.id) ?? p.default, p, range);
      if (input.reflect) n = -n;
      const v = (n * input.weight) / 100;
      if (input.source === "x") pose.t[0] += v;
      else if (input.source === "y") pose.t[1] += v;
      else pose.angle += v;
    });
    return pose;
  }

  // ---- 動的な物理の文脈 (spec/physics.md#dynamic-physics) ----

  private context(r: number, deformers: Map<string, DeformerState> | null): RigContext {
    const rig = this.settings.rigs[r];
    const ctx: RigContext = {
      anchor: [0, 0],
      frameAngle: 0,
      angleRef: rig.outputs.map(() => null),
      angleSign: rig.outputs.map(() => 1),
      colliders: [],
    };
    if (!this.dynamic || deformers === null) return ctx;
    const world = (id: string) => deformers.get(id)?.world;
    // 根のアンカー
    const first = this.targets[r].find(
      (t) => t.kind === "deformer_angle" || t.kind === "deformer_offset",
    );
    const anchorId =
      first && "id" in first
        ? first.id
        : rig.frame !== null && deformers.has(rig.frame)
          ? rig.frame
          : null;
    if (anchorId !== null) {
      const F = world(anchorId);
      const p0 = rig.particles[0]?.position ?? [0, 0];
      if (F) ctx.anchor = sub(apply(F, p0), p0);
    }
    // 重力の座標系
    if (rig.frame !== null) {
      const F = world(rig.frame);
      if (F) {
        const g = normalize(this.settings.gravity, [0, 1]);
        const isWarp = this.model.deformers.find((d) => d.id === rig.frame)?.kind.type === "warp";
        const J = jacobian(F, isWarp ? [0.5, 0.5] : [0, 0]);
        ctx.frameAngle = signedAngle(g, [J[0] * g[0] + J[1] * g[1], J[2] * g[0] + J[3] * g[1]]);
      }
    }
    // 角度の出力の基準と鏡映 (spec/physics.md#angle-outputs)
    this.targets[r].forEach((t, k) => {
      if (t.kind !== "deformer_angle") return;
      const d = this.model.deformers.find((x) => x.id === t.id);
      const state = deformers.get(t.id);
      if (!d || !state?.origin) return;
      const parentWorld =
        d.parent === null ? { kind: "identity" as const } : (world(d.parent) ?? null);
      if (parentWorld === null) return;
      const J = jacobian(parentWorld, state.origin);
      const g = this.settings.gravity;
      ctx.angleRef[k] = [J[0] * g[0] + J[1] * g[1], J[2] * g[0] + J[3] * g[1]];
      ctx.angleSign[k] = J[0] * J[3] - J[1] * J[2] < 0 ? -1 : 1;
    });
    // 当たり判定: 形を 1 回の更新の間は固定する
    for (const id of rig.colliders) {
      const c = this.settings.colliders.find((x) => x.id === id);
      if (!c || c.shape.type === "unknown") continue;
      let F: DeformerState["world"] = { kind: "identity" };
      if (c.attach !== null) {
        const w = world(c.attach);
        if (!w) continue;
        F = w;
      }
      if (c.shape.type === "circle") {
        ctx.colliders.push({ kind: "circle", c: apply(F, c.shape.center), r: c.shape.radius });
      } else {
        ctx.colliders.push({
          kind: "capsule",
          a: apply(F, c.shape.a),
          b: apply(F, c.shape.b),
          r: c.shape.radius,
        });
      }
    }
    return ctx;
  }

  // ---- 時間を進める (spec/physics.md#advancing-time) ----

  /**
   * 1 回の更新。`values` を書き換え (パラメータへの出力)、`this.offsets` を作り直す。
   * `deformers` は動的な物理の 1 段目のデフォーマの結果 (使わないなら null)。
   */
  update(dt: number, values: ParameterValues, deformers: Map<string, DeformerState> | null): void {
    const s = this.settings;
    const fps = Number.isFinite(s.fps) && s.fps > 0 ? s.fps : 60;
    const step = 1 / fps;
    const tau = step * 30;
    const rigs = s.rigs;
    const ctxs = rigs.map((_, r) => this.context(r, this.usesDynamic ? deformers : null));

    // 1. 目標の姿勢
    const targets: Pose[] = rigs.map((_, r) => {
      const t = this.rootPose(r, values, false);
      t.t = add(t.t, ctxs[r].anchor);
      let fa = ctxs[r].frameAngle;
      if (this.initialized) {
        const prev = this.rigs[r].pose.frame_angle;
        fa = prev + wrap180(fa - prev);
      }
      t.angle += fa;
      t.frame_angle = fa;
      return t;
    });

    // 中継の写し (spec/physics.md#rig-order-and-relays)
    const start = new Map(values);
    const relay = new Map(values);
    const resetRelay = () => {
      for (const id of this.relayTargets) relay.set(id, start.get(id) ?? 0);
    };
    const withRelay = (r: number, external: Pose): Pose => {
      const ri = this.rootPose(r, relay, true);
      return {
        t: add(external.t, ri.t),
        angle: external.angle + ri.angle,
        frame_angle: external.frame_angle,
      };
    };
    const writeRelay = (r: number) => {
      rigs[r].outputs.forEach((o, k) => {
        const t = this.targets[r][k];
        const v = this.rigs[r].current[k];
        if (t.kind === "parameter" && typeof v === "number") this.writeParameter(relay, t.id, o, v);
      });
    };

    // 2. 最初の更新: 静止の姿勢に置く
    if (!this.initialized) {
      this.initialized = true;
      this.accumulator = 0;
      resetRelay();
      this.rigs = [];
      rigs.forEach((rig, r) => {
        const pose = withRelay(r, targets[r]);
        const position = this.restPose(rig, pose, ctxs[r].colliders);
        const n = rig.particles.length;
        const state: RigState = {
          position,
          last: position.map((p) => [p[0], p[1]]),
          velocity: Array.from({ length: n }, (): Vec2 => [0, 0]),
          pose,
          external: targets[r],
          previous: [],
          current: [],
        };
        this.rigs.push(state);
        state.current = this.outputs(r, ctxs[r]);
        state.previous = state.current;
        writeRelay(r);
      });
    }

    // 3. 蓄積
    if (Number.isFinite(dt) && dt > 0) this.accumulator += dt;
    const whole = Math.floor(this.accumulator / step);
    let n: number;
    if (whole <= 8) {
      this.accumulator -= whole * step;
      n = whole;
    } else {
      // 残りの時間は捨てる。引き算ではなく割り算の正確な余り ([0, step)) にする。
      // JavaScript の `%` は C の fmod と同じく正確な余りを返す。
      this.accumulator %= step;
      n = 8;
    }

    // 4. 固定の刻み
    const starts = this.rigs.map((st) => st.external);
    for (let k = 1; k <= n; k++) {
      resetRelay();
      rigs.forEach((rig, r) => {
        const st = this.rigs[r];
        const f = k / n;
        const a = starts[r];
        const b = targets[r];
        const external: Pose = {
          t: [lerp(a.t[0], b.t[0], f), lerp(a.t[1], b.t[1], f)],
          angle: lerp(a.angle, b.angle, f),
          frame_angle: lerp(a.frame_angle, b.frame_angle, f),
        };
        const pose = withRelay(r, external);
        this.stepChain(rig, st, pose, pose.angle - st.pose.angle, tau, ctxs[r].colliders);
        st.pose = pose;
        st.external = external;
        st.previous = st.current;
        st.current = this.outputs(r, ctxs[r]);
        writeRelay(r);
      });
    }

    // 5. 出力を書く
    const alpha = clamp(this.accumulator / step, 0, 1);
    const offsets = emptyOffsets();
    rigs.forEach((rig, r) => {
      const st = this.rigs[r];
      rig.outputs.forEach((o, k) => {
        const t = this.targets[r][k];
        const prev = st.previous[k];
        const cur = st.current[k];
        if (prev === null || cur === null || t.kind === "ignored") return;
        const w = clamp(o.weight / 100, 0, 1);
        if (typeof prev === "number" && typeof cur === "number") {
          const v = lerp(prev, cur, alpha);
          if (t.kind === "parameter") this.writeParameter(values, t.id, o, v);
          else if (t.kind === "deformer_angle") {
            offsets.angles.set(t.id, (offsets.angles.get(t.id) ?? 0) + v * w);
          }
          return;
        }
        if (typeof prev === "number" || typeof cur === "number") return;
        const v: Vec2 = [lerp(prev[0], cur[0], alpha), lerp(prev[1], cur[1], alpha)];
        if (t.kind === "deformer_offset") {
          offsets.origins.set(t.id, add(offsets.origins.get(t.id) ?? [0, 0], mul(v, w)));
        } else if (t.kind === "points" || t.kind === "vertices") {
          const map = t.kind === "points" ? offsets.points : offsets.vertices;
          const count = this.vertexCount(t);
          const acc = map.get(t.id) ?? Array.from({ length: count }, (): Vec2 => [0, 0]);
          for (let i = 0; i < count && i < t.weights.length; i++) {
            acc[i] = add(acc[i], mul(v, w * t.weights[i]));
          }
          map.set(t.id, acc);
        }
      });
    });
    this.offsets = offsets;
  }

  private vertexCount(t: { kind: "points" | "vertices"; id: string }): number {
    if (t.kind === "vertices") {
      return this.model.meshes.find((m) => m.id === t.id)?.uvs.length ?? 0;
    }
    const d = this.model.deformers.find((x) => x.id === t.id);
    return d?.kind.type === "warp" ? (d.kind.rows + 1) * (d.kind.cols + 1) : 0;
  }

  /** パラメータへの書き込み (spec/physics.md#writing-to-parameters)。 */
  private writeParameter(values: ParameterValues, id: string, o: M.PhysicsOutput, v: number): void {
    const p = this.params.get(id);
    if (!p) return;
    const w = clamp(o.weight / 100, 0, 1);
    const cur = values.get(id) ?? p.default;
    const next =
      o.blend === "add"
        ? clamp(cur + v * w, p.min, p.max)
        : clamp(cur + (clamp(v, p.min, p.max) - cur) * w, p.min, p.max);
    values.set(id, next);
  }

  // ---- 振り子 (spec/physics.md#pendulum-step, #rest-pose) ----

  private forces(pose: Pose): { g: Vec2; w: Vec2 } {
    return {
      g: rotate(this.settings.gravity, pose.angle),
      w: rotate(this.settings.wind, pose.frame_angle),
    };
  }

  private restSegment(rig: M.PhysicsRig, i: number, pose: Pose): Vec2 {
    const { g, w } = this.forces(pose);
    const p = rig.particles[i];
    return mul(normalize(add(mul(g, p.acceleration), w), restDir(rig, i)), p.radius);
  }

  /** 当たり判定なしの静止の位置 (根から)。`rest_offset(i)`。 */
  private restOffsets(rig: M.PhysicsRig, pose: Pose): Vec2[] {
    const out: Vec2[] = [[0, 0]];
    for (let i = 1; i < rig.particles.length; i++) {
      out.push(add(out[i - 1], this.restSegment(rig, i, pose)));
    }
    return out;
  }

  private restPose(rig: M.PhysicsRig, pose: Pose, colliders: Core[]): Vec2[] {
    if (rig.particles.length === 0) return [];
    const position: Vec2[] = [add(pose.t, rig.particles[0].position)];
    for (let i = 1; i < rig.particles.length; i++) {
      const parent = position[i - 1];
      const from = add(parent, mul(restDir(rig, i), rig.particles[i].radius));
      const q = add(parent, this.restSegment(rig, i, pose));
      position.push(collide(q, parent, from, colliders, rig.particles[i].collision_radius));
    }
    return position;
  }

  private stepChain(
    rig: M.PhysicsRig,
    st: RigState,
    pose: Pose,
    rotationDelta: number,
    tau: number,
    colliders: Core[],
  ): void {
    if (rig.particles.length === 0) return;
    const { g, w } = this.forces(pose);
    st.last[0] = st.position[0];
    st.position[0] = add(pose.t, rig.particles[0].position);
    for (let i = 1; i < rig.particles.length; i++) {
      const p = rig.particles[i];
      const parent = st.position[i - 1];
      const parentLast = st.last[i - 1];
      const fromDir = normalize(sub(st.position[i], parentLast), restDir(rig, i));
      const last = st.position[i];
      const h = tau * p.delay;
      let q = add(parent, rotate(sub(st.position[i], parent), rotationDelta * 0.2));
      q = add(add(q, mul(st.velocity[i], h)), mul(add(mul(g, p.acceleration), w), h * h));
      q = add(parent, mul(normalize(sub(q, parent), restDir(rig, i)), p.radius));
      q = collide(q, parent, add(parent, mul(fromDir, p.radius)), colliders, p.collision_radius);
      st.velocity[i] = h > 0 ? mul(sub(q, last), Math.max(p.mobility, 0) ** tau / h) : [0, 0];
      st.last[i] = last;
      st.position[i] = q;
    }
  }

  // ---- 出力の値 (spec/physics.md#output-values, #displacement-outputs) ----

  private outputs(r: number, ctx: RigContext): OutputValue[] {
    const rig = this.settings.rigs[r];
    const st = this.rigs[r];
    const pos = st.position;
    const count = rig.particles.length;
    let rest: Vec2[] | null = null;
    const restOf = () => {
      rest ??= this.restOffsets(rig, st.pose);
      return rest;
    };
    return rig.outputs.map((o, k): OutputValue => {
      const t = this.targets[r][k];
      const i = o.particle;
      if (t.kind === "ignored" || o.source === "unknown" || i === 0 || i >= count) return null;
      const sign = o.reflect ? -1 : 1;
      if (t.kind === "parameter" || t.kind === "deformer_angle") {
        const reference =
          t.kind === "deformer_angle"
            ? (ctx.angleRef[k] ?? this.settings.gravity)
            : this.settings.gravity;
        const quantity = (p: Vec2[]): number => {
          const segment = sub(p[i], p[i - 1]);
          const parent = i >= 2 ? sub(p[i - 1], p[i - 2]) : reference;
          if (o.source === "x") return segment[0];
          if (o.source === "y") return segment[1];
          return signedAngle(parent, segment);
        };
        let s = quantity(pos);
        if (t.kind === "parameter" && o.blend === "add") {
          s -= quantity(restOf());
        }
        let value = s * o.scale * sign;
        if (t.kind === "deformer_angle") value *= ctx.angleSign[k];
        return value;
      }
      // 変位
      if (o.source === "angle") return null;
      const d = sub(sub(pos[i], pos[0]), restOf()[i]);
      const v: Vec2 = o.source === "x" ? [d[0], 0] : [0, d[1]];
      return mul(v, o.scale * sign);
    });
  }
}

/** `normalize_input` (spec/physics.md#inputs)。 */
function normalizeInput(value: number, p: M.Parameter, range: M.NormalizationRange): number {
  if (value > p.default && p.max > p.default) {
    const t = Math.min((value - p.default) / (p.max - p.default), 1);
    return lerp(range.default, range.max, t);
  }
  if (value < p.default && p.default > p.min) {
    const t = Math.min((p.default - value) / (p.default - p.min), 1);
    return lerp(range.default, range.min, t);
  }
  return range.default;
}

/** `rest_dir(i)`。 */
function restDir(rig: M.PhysicsRig, i: number): Vec2 {
  return normalize(sub(rig.particles[i].position, rig.particles[i - 1].position), [0, 1]);
}

// ---- 当たり判定 (spec/physics.md#colliders) ----

/** 点 `p` に最も近い核の上の点。 */
function closestOnCore(core: Core, p: Vec2): Vec2 {
  if (core.kind === "circle") return core.c;
  const ab = sub(core.b, core.a);
  const l2 = ab[0] * ab[0] + ab[1] * ab[1];
  if (l2 === 0) return core.a;
  const t = clamp(((p[0] - core.a[0]) * ab[0] + (p[1] - core.a[1]) * ab[1]) / l2, 0, 1);
  return add(core.a, mul(ab, t));
}

const distPoint = (core: Core, p: Vec2): number => len(sub(p, closestOnCore(core, p)));

/** 点と線分の距離。 */
function distPointSegment(p: Vec2, a: Vec2, b: Vec2): number {
  const ab = sub(b, a);
  const l2 = ab[0] * ab[0] + ab[1] * ab[1];
  const t = l2 === 0 ? 0 : clamp(((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / l2, 0, 1);
  return len(sub(p, add(a, mul(ab, t))));
}

/** 2 つの線分の距離。 */
function distSegments(p: Vec2, q: Vec2, a: Vec2, b: Vec2): number {
  const cross = (u: Vec2, v: Vec2) => u[0] * v[1] - u[1] * v[0];
  const d1 = sub(q, p);
  const d2 = sub(b, a);
  const den = cross(d1, d2);
  if (den !== 0) {
    const s = cross(sub(a, p), d2) / den;
    const t = cross(sub(a, p), d1) / den;
    if (s >= 0 && s <= 1 && t >= 0 && t <= 1) return 0;
  }
  return Math.min(
    distPointSegment(p, a, b),
    distPointSegment(q, a, b),
    distPointSegment(a, p, q),
    distPointSegment(b, p, q),
  );
}

/** 線分 `P–q` と核の距離。 */
function distSegment(core: Core, P: Vec2, q: Vec2): number {
  if (core.kind === "circle") return distPointSegment(core.c, P, q);
  return distSegments(P, q, core.a, core.b);
}

function blockedBy(core: Core, q: Vec2, P: Vec2, mu: number): boolean {
  if (distPoint(core, q) < core.r + mu) return true;
  return distPoint(core, P) >= core.r && distSegment(core, P, q) < core.r;
}

function pushOut(core: Core, q: Vec2, mu: number): Vec2 {
  if (!(distPoint(core, q) < core.r + mu)) return q;
  const c = closestOnCore(core, q);
  let dir: Vec2;
  const d = sub(q, c);
  if (d[0] === 0 && d[1] === 0) {
    if (core.kind === "circle") dir = [0, -1];
    else {
      const ab = sub(core.b, core.a);
      dir = normalize([ab[1], -ab[0]], [0, -1]);
    }
  } else dir = normalize(d, [0, -1]);
  return add(c, mul(dir, core.r + mu));
}

/** `collide(q, parent P, from)`。当たり判定が無ければ `q` のまま。 */
export function collide(
  q: Vec2,
  P: Vec2,
  from: Vec2,
  colliders: Core[],
  collisionRadius: number,
): Vec2 {
  if (colliders.length === 0) return q;
  const mu = Math.max(collisionRadius, 0);
  const L = len(sub(q, P));
  const near = colliders.filter((c) => distPoint(c, P) < L + c.r + mu);
  if (near.length === 0) return q;
  const pushAll = (x: Vec2): Vec2 => {
    let y = x;
    for (const c of near) y = pushOut(c, y, mu);
    return y;
  };
  if (q[0] === P[0] && q[1] === P[1]) return pushAll(q);
  const target = normalize(sub(q, P), [0, 1]);
  const base = normalize(sub(from, P), target);
  const at = (theta: number): Vec2 => add(P, mul(rotRad(base, theta), L));
  const blocked = (theta: number): boolean => {
    const x = at(theta);
    return near.some((c) => blockedBy(c, x, P, mu));
  };
  const minR = Math.min(...near.map((c) => c.r));
  const step = Math.min(Math.max(minR, 0.5) / L, 10 * DEG);
  const end = signedAngleRad(base, target);
  const bisect = (free0: number, hit0: number): number => {
    let free = free0;
    let hit = hit0;
    for (let k = 0; k < 16; k++) {
      const mid = (free + hit) / 2;
      if (blocked(mid)) hit = mid;
      else free = mid;
    }
    return free;
  };
  let start = 0;
  if (blocked(0)) {
    let found: number | null = null;
    const kMax = Math.ceil(Math.PI / step);
    search: for (let k = 1; k <= kMax; k++) {
      for (const sgn of [1, -1]) {
        const theta = sgn * Math.min(step * k, Math.PI);
        if (!blocked(theta)) {
          found = bisect(theta, sgn * step * (k - 1));
          break search;
        }
      }
    }
    if (found === null) return pushAll(q);
    start = found;
  }
  const span = wrapPi(end - start);
  const samples = Math.max(Math.ceil(Math.abs(span) / step), 1);
  let free = start;
  for (let k = 1; k <= samples; k++) {
    const theta = start + (span * k) / samples;
    if (blocked(theta)) return at(bisect(free, theta));
    free = theta;
  }
  return q;
}
