// モーション・表情・効果 (目パチ・呼吸・リップシンク)・ポーズ・パーツの不透明度の上書き
// (spec/motion.md)。1 フレームの順は frame.ts。

import type { ParameterValues, PartOverrides } from "./evaluate.ts";
import type * as M from "./types.ts";

const clamp = (x: number, lo: number, hi: number): number => Math.min(Math.max(x, lo), hi);
const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;

/** `easing(x)` (spec/motion.md#overview)。 */
export const easing = (x: number): number => 0.5 - 0.5 * Math.cos(Math.PI * clamp(x, 0, 1));

/** パラメータに書く。範囲に切り詰め、知らない ID は無視する (spec/motion.md#overview)。 */
export function writeParameter(
  model: M.Model,
  values: ParameterValues,
  id: string,
  f: (v: number) => number,
): void {
  const p = model.parameters.find((q) => q.id === id);
  if (!p) return;
  values.set(id, clamp(f(values.get(id) ?? p.default), p.min, p.max));
}

// ---- 曲線 (spec/motion.md#curves) ----

/** 曲線の時刻 `t` の値。値が無ければ null。 */
export function curveValue(segments: M.Segment[], t: number): number | null {
  const first = segments[0];
  if (first?.type !== "start") return null;
  // 2 つ目以降に start がある曲線は規則を破る。仕様は値を決めず、全体を無視してよい (MAY) とするので、無視する。
  if (segments.slice(1).some((s) => s.type === "start")) return null;
  if (t <= first.p.t) return first.p.v;
  let p0 = first.p;
  for (const s of segments.slice(1)) {
    if (s.type === "start") return null;
    const to = s.to;
    if (t <= to.t) {
      const u = to.t <= p0.t ? 1 : (t - p0.t) / (to.t - p0.t);
      switch (s.type) {
        case "linear":
          return p0.v + (to.v - p0.v) * u;
        case "stepped":
          return u < 1 ? p0.v : to.v;
        case "inverse_stepped":
          return u > 0 ? to.v : p0.v;
        case "bezier":
          return bezierValue(p0, s.c1, s.c2, to, t);
      }
    }
    p0 = to;
  }
  return p0.v;
}

function bezierValue(p0: M.Point, c1: M.Point, c2: M.Point, to: M.Point, t: number): number {
  if (to.t <= p0.t) return to.v;
  const span = to.t - p0.t;
  const x1 = clamp((c1.t - p0.t) / span, 0, 1);
  const x2 = clamp((c2.t - p0.t) / span, 0, 1);
  const x = clamp((t - p0.t) / span, 0, 1);
  let s: number;
  if (x <= 0) s = 0;
  else if (x >= 1) s = 1;
  else {
    // b(s) は [0, 1] で単調なので二分法で解く (仕様は |b(s) − x| ≤ 1e-6 を求める)
    const b = (s: number) => 3 * (1 - s) ** 2 * s * x1 + 3 * (1 - s) * s * s * x2 + s ** 3;
    let lo = 0;
    let hi = 1;
    for (let k = 0; k < 60; k++) {
      const mid = (lo + hi) / 2;
      if (b(mid) < x) lo = mid;
      else hi = mid;
    }
    s = (lo + hi) / 2;
  }
  const r = 1 - s;
  return r ** 3 * p0.v + 3 * r * r * s * c1.v + 3 * r * s * s * c2.v + s ** 3 * to.v;
}

// ---- フェード (spec/motion.md#motion-playback) ----

/** フェードの重み。`end` が null なら終わりが無い。 */
export function fadeWeight(elapsed: number, end: number | null, fi: number, fo: number): number {
  const wIn = fi > 0 ? easing(elapsed / fi) : 1;
  let wOut: number;
  if (end === null) wOut = 1;
  else if (fo > 0) wOut = easing((end - elapsed) / fo);
  else wOut = end - elapsed > 0 ? 1 : 0;
  return wIn * wOut;
}

// ---- モーションの再生 (spec/motion.md#motion-playback) ----

export type Priority = "idle" | "normal" | "force";
const RANK: Record<Priority, number> = { idle: 0, normal: 1, force: 2 };

export class PlayingMotion {
  readonly motion: M.Motion;
  readonly priority: Priority;
  elapsed = 0;
  time = 0;
  end: number | null;
  stopped = false;
  /** 次の範囲の始まりを含めるか (開始直後と、巻き戻した直後)。 */
  private inclusive = true;

  constructor(motion: M.Motion, priority: Priority) {
    this.motion = motion;
    this.priority = priority;
    this.end = motion.looping ? null : Math.max(motion.duration, 0);
  }

  get finished(): boolean {
    return this.end !== null && this.elapsed >= this.end;
  }

  stop(): void {
    const e = this.elapsed + Math.max(this.motion.fade_out, 0);
    this.end = this.end === null ? e : Math.min(this.end, e);
    this.stopped = true;
  }

  /** 時間を進め、発火したイベントの value を返す。 */
  advance(dt0: number): string[] {
    const fired: string[] = [];
    if (this.finished || !(dt0 > 0)) return fired;
    const m = this.motion;
    let dt = dt0;
    if (this.end !== null) dt = Math.min(dt, this.end - this.elapsed);
    this.elapsed += dt;
    const fire = (from: number, to: number) => {
      const incl = this.inclusive;
      this.inclusive = false;
      const hits = m.events
        .map((e, i) => ({ e, i }))
        .filter(({ e }) => (incl ? e.t >= from : e.t > from) && e.t <= to)
        .sort((a, b) => a.e.t - b.e.t || a.i - b.i);
      for (const { e } of hits) fired.push(e.value);
    };
    if (!m.looping || m.duration <= 0) {
      const to = Math.min(this.time + dt, Math.max(m.duration, 0));
      fire(this.time, to);
      this.time = to;
    } else {
      let remaining = dt;
      for (;;) {
        if (this.time + remaining < m.duration) {
          fire(this.time, this.time + remaining);
          this.time += remaining;
          break;
        }
        fire(this.time, m.duration);
        remaining -= m.duration - this.time;
        this.time = 0;
        this.inclusive = true;
        if (remaining <= 0) break;
      }
    }
    return fired;
  }

  /** 曲線ごとの `(曲線, 値, 重み)`。終わったモーションは何も返さない。 */
  *applied(): Generator<{ curve: M.Curve; value: number; weight: number }> {
    if (this.finished) return;
    for (const curve of this.motion.curves) {
      const value = curveValue(curve.segments, this.time);
      if (value === null) continue;
      const fi = curve.fade_in ?? this.motion.fade_in;
      const fo = curve.fade_out ?? this.motion.fade_out;
      yield { curve, value, weight: fadeWeight(this.elapsed, this.end, fi, fo) };
    }
  }
}

/** 効果のグループの強さ (spec/motion.md#effect-groups)。 */
export interface EffectStrength {
  EyeBlink: number;
  LipSync: number;
}

/** 1 フレームのモーションの結果。 */
export interface MotionFrame {
  events: string[];
  strength: EffectStrength;
  /** パーツ ID → 開始順の `(c, w)` の要求。 */
  requests: Map<string, [number, number][]>;
}

/** 複数のモーションの再生 (spec/motion.md#playing-several-motions)。 */
export class MotionManager {
  playing: PlayingMotion[] = [];
  private current: PlayingMotion | null = null;

  /** 開始を求める。受け付けたら true。 */
  start(motion: M.Motion, priority: Priority): boolean {
    const cur = this.current;
    if (cur && !cur.stopped && !cur.finished && RANK[priority] < RANK[cur.priority]) return false;
    for (const p of this.playing) p.stop();
    const m = new PlayingMotion(motion, priority);
    this.playing.push(m);
    this.current = m;
    return true;
  }

  stopAll(): void {
    for (const p of this.playing) p.stop();
    this.current = null;
  }

  frame(model: M.Model, values: ParameterValues, dt: number): MotionFrame {
    const out: MotionFrame = {
      events: [],
      strength: { EyeBlink: 1, LipSync: 1 },
      requests: new Map(),
    };
    for (const m of this.playing) {
      out.events.push(...m.advance(dt));
      for (const { curve, value, weight } of m.applied()) {
        const id = curve.target.id;
        switch (curve.target.kind) {
          case "parameter":
            writeParameter(model, values, id, (v) => lerp(v, value, weight));
            break;
          case "part_opacity": {
            if (!model.parts.some((p) => p.id === id)) break;
            const list = out.requests.get(id) ?? [];
            list.push([value, weight]);
            out.requests.set(id, list);
            break;
          }
          case "effect_group":
            if (id === "EyeBlink" || id === "LipSync") {
              out.strength[id] = lerp(out.strength[id], value, weight);
            }
            break;
        }
      }
    }
    if (this.current?.finished) this.current = null;
    this.playing = this.playing.filter((m) => !m.finished);
    return out;
  }
}

// ---- 表情 (spec/motion.md#expressions) ----

class ActiveExpression {
  readonly expression: M.Expression;
  elapsed = 0;
  end: number | null = null;

  constructor(expression: M.Expression) {
    this.expression = expression;
  }

  get finished(): boolean {
    return this.end !== null && this.elapsed >= this.end;
  }

  stop(): void {
    const e = this.elapsed + Math.max(this.expression.fade_out, 0);
    this.end = this.end === null ? e : Math.min(this.end, e);
  }
}

export class ExpressionManager {
  active: ActiveExpression[] = [];

  /** すべての表情を止める (フェードアウトで終わる)。 */
  stopAll(): void {
    for (const a of this.active) a.stop();
  }

  /** すべて止めてから足す。 */
  set(expression: M.Expression): void {
    this.stopAll();
    this.add(expression);
  }

  add(expression: M.Expression): void {
    this.active.push(new ActiveExpression(expression));
  }

  frame(model: M.Model, values: ParameterValues, dt: number): void {
    for (const a of this.active) {
      if (dt > 0) {
        a.elapsed += dt;
        if (a.end !== null) a.elapsed = Math.min(a.elapsed, a.end);
      }
      const e = a.expression;
      const w = fadeWeight(a.elapsed, a.end, e.fade_in, e.fade_out);
      for (const item of e.parameters) {
        writeParameter(model, values, item.parameter, (v) => {
          switch (item.op) {
            case "add":
              return v + item.value * w;
            case "multiply":
              return v * (1 + (item.value - 1) * w);
            case "overwrite":
              return v + (item.value - v) * w;
          }
        });
      }
    }
    this.active = this.active.filter((a) => !a.finished);
  }
}

// ---- 目パチ (spec/motion.md#eye-blink) ----

const MASK64 = 2n ** 64n - 1n;

/** SplitMix64 の 1 回の引き。`[新しい状態, u]` を返す。 */
export function splitMix64(state: bigint): [bigint, number] {
  const s = (state + 0x9e3779b97f4a7c15n) & MASK64;
  let z = s;
  z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK64;
  z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & MASK64;
  z = z ^ (z >> 31n);
  return [s, Number(z >> 40n) / 2 ** 24];
}

type BlinkPhase = "open" | "closing" | "closed" | "opening";
const NEXT: Record<BlinkPhase, BlinkPhase> = {
  open: "closing",
  closing: "closed",
  closed: "opening",
  opening: "open",
};

export class EyeBlink {
  readonly settings: M.EyeBlink;
  phase: BlinkPhase = "open";
  t = 0;
  interval: number;
  private state: bigint;
  /** 引いた間隔の記録 (テスト用)。 */
  readonly intervals: number[] = [];

  constructor(settings: M.EyeBlink) {
    this.settings = settings;
    this.state = settings.seed;
    this.interval = this.draw();
  }

  private draw(): number {
    const [s, u] = splitMix64(this.state);
    this.state = s;
    const interval = Math.fround(this.settings.interval * Math.fround(0.5 + u));
    this.intervals.push(interval);
    return interval;
  }

  private length(phase: BlinkPhase): number {
    switch (phase) {
      case "open":
        return this.interval;
      case "closing":
        return this.settings.closing;
      case "closed":
        return this.settings.closed;
      case "opening":
        return this.settings.opening;
    }
  }

  advance(dt0: number): void {
    if (!(dt0 > 0) || !(this.settings.interval > 0)) return;
    let dt = dt0;
    for (;;) {
      const len = this.length(this.phase);
      if (this.t + dt < len) {
        this.t += dt;
        return;
      }
      dt -= Math.max(len - this.t, 0);
      this.t = 0;
      this.phase = NEXT[this.phase];
      if (this.phase === "open") this.interval = this.draw();
    }
  }

  get openness(): number {
    const { closing, opening } = this.settings;
    if (this.phase === "closing") return closing === 0 ? 1 : 1 - Math.min(this.t / closing, 1);
    if (this.phase === "closed") return 0;
    if (this.phase === "opening") return opening === 0 ? 1 : Math.min(this.t / opening, 1);
    return 1;
  }

  apply(model: M.Model, values: ParameterValues, s: number): void {
    const o = this.openness;
    for (const id of this.settings.parameters) {
      writeParameter(model, values, id, (v) => v * (1 + (o - 1) * s));
    }
  }
}

// ---- 呼吸 (spec/motion.md#breathing) ----

export class Breath {
  readonly settings: M.BreathParameter[];
  readonly phase: number[];

  constructor(settings: M.BreathParameter[]) {
    this.settings = settings;
    this.phase = settings.map(() => 0);
  }

  advance(dt: number): void {
    if (!(dt > 0)) return;
    this.settings.forEach((b, i) => {
      if (!(b.cycle > 0)) {
        this.phase[i] = 0;
        return;
      }
      const x = this.phase[i] + dt;
      this.phase[i] = x - b.cycle * Math.floor(x / b.cycle);
    });
  }

  apply(model: M.Model, values: ParameterValues): void {
    this.settings.forEach((b, i) => {
      const wave = b.cycle > 0 ? b.peak * Math.sin((2 * Math.PI * this.phase[i]) / b.cycle) : 0;
      writeParameter(model, values, b.parameter, (v) => v + b.weight * (b.offset + wave));
    });
  }
}

// ---- ポーズとパーツの不透明度 (spec/motion.md#pose, #part-opacity) ----

/** 要求を 1 つの上書き `(v, W)` にまとめる。 */
export function combineRequests(requests: [number, number][]): { value: number; weight: number } {
  let v = 0;
  let W = 0;
  for (const [c, w] of requests) {
    const W2 = 1 - (1 - W) * (1 - w);
    if (W2 > 0) v = (v * W * (1 - w) + c * w) / W2;
    W = W2;
  }
  return { value: v, weight: W };
}

export class PoseState {
  readonly pose: M.Pose;
  readonly shown: number[];
  readonly opacity: number[][];

  constructor(pose: M.Pose) {
    this.pose = pose;
    this.shown = pose.groups.map(() => 0);
    this.opacity = pose.groups.map((g) => g.entries.map((_, i) => (i === 0 ? 1 : 0)));
  }

  /** `part` のエントリをそのグループの表示するエントリにする。ポーズに無いパーツなら何もしない。 */
  show(part: string): void {
    this.pose.groups.forEach((g, gi) => {
      const i = g.entries.findIndex((e) => e.part === part);
      if (i >= 0) this.shown[gi] = i;
    });
  }

  /** 各エントリの `part` (リンクは含まない)。 */
  entryParts(): Set<string> {
    return new Set(this.pose.groups.flatMap((g) => g.entries.map((e) => e.part)));
  }

  /** `requests` はパーツ ID → ポーズの要求 `v × W`。上書きを `overrides` に書く。 */
  frame(requests: Map<string, number>, dt: number, overrides: PartOverrides): void {
    const fade = this.pose.fade_duration;
    this.pose.groups.forEach((g, gi) => {
      let best = -1;
      let bestValue = 0;
      g.entries.forEach((e, i) => {
        const r = requests.get(e.part);
        if (r !== undefined && r > 0 && (best < 0 || r > bestValue)) {
          best = i;
          bestValue = r;
        }
      });
      if (best >= 0) this.shown[gi] = best;
      const op = this.opacity[gi];
      const s = this.shown[gi];
      op[s] = fade <= 0 ? 1 : Math.min(op[s] + Math.max(dt, 0) / fade, 1);
      op.forEach((o, i) => {
        if (i !== s) op[i] = Math.min(o, 1 - op[s]);
      });
      g.entries.forEach((e, i) => {
        for (const part of [e.part, ...e.links]) overrides.set(part, { value: op[i], weight: 1 });
      });
    });
  }
}
