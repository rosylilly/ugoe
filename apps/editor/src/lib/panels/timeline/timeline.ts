// タイムライン (モーションの編集) の、DOM に依存しない補助。Vitest で検査する (timeline.test.ts)。
// コマンドの規則は docs/editor.md#モーションと表情の編集、カーブの式は docs/spec/motion.md#カーブ。
import type {
  Command,
  Curve,
  CurveTarget,
  Motion,
  Parameter,
  Point,
  Segment,
  SegmentShape,
} from "../../ipc";

/** 区間の種類 (`Segment` の `type` のうち `start` 以外)。 */
export type SegmentKind = Exclude<Segment["type"], "start">;

export const SEGMENT_KINDS: { kind: SegmentKind; label: string; description: string }[] = [
  { kind: "linear", label: "Linear", description: "a straight line" },
  { kind: "bezier", label: "Bezier", description: "a bezier curve with handles" },
  { kind: "stepped", label: "Step", description: "a step at this key" },
  {
    kind: "inverse_stepped",
    label: "Inv. step",
    description: "a step right after the previous key",
  },
];

/** キーを打つ・動かすときに揃えるフレームの数 (1 秒あたり)。 */
export const FPS = 30;

/** 新しいモーションの長さ (秒)。 */
export const DEFAULT_DURATION = 2;

/** カーブの対象を 1 つの文字列にする (表の行や選択のキー)。 */
export function targetKey(target: CurveTarget): string {
  return `${target.kind}:${target.id}`;
}

/** 区間の終点 (`start` は始点)。 */
export function endPoint(segment: Segment): Point {
  return segment.type === "start" ? segment.p : segment.to;
}

/** カーブのキー (各区間の終点) を時刻の順に。 */
export function curveKeys(curve: Curve): Point[] {
  return curve.segments.map(endPoint);
}

/** 時刻 `t` のキーで終わる区間 (最初のキーなら null)。 */
export function segmentEndingAt(curve: Curve, t: number): Segment | null {
  const index = curve.segments.findIndex((s) => endPoint(s).t === t);
  return index > 0 ? (curve.segments[index] ?? null) : null;
}

/** 対象のカーブ。 */
export function findCurve(motion: Motion, target: CurveTarget): Curve | null {
  const key = targetKey(target);
  return motion.curves.find((c) => targetKey(c.target) === key) ?? null;
}

/** 時刻をフレームに揃える (1/FPS 秒の倍数)。浮動小数点の誤差を丸めて返す。 */
export function snapToFrame(t: number, fps = FPS): number {
  return roundTime(Math.round(t * fps) / fps);
}

/** 表示と入力のために時刻を丸める (1e-4 秒)。 */
export function roundTime(t: number): number {
  return Math.round(t * 10000) / 10000;
}

/**
 * 再生で時刻を `dt` 進める。ループなら長さで折り返し、そうでなければ終わりで止めて `ended` にする。
 */
export function advance(
  time: number,
  dt: number,
  duration: number,
  loop: boolean,
): { time: number; ended: boolean } {
  const next = time + dt;
  if (duration <= 0) return { time: 0, ended: !loop };
  if (next < duration) return { time: next, ended: false };
  if (loop) return { time: next % duration, ended: false };
  return { time: duration, ended: true };
}

// --- 表示 ---

/** 時刻と値を、描画の座標 (px) に写す。値は上が大きい。 */
export interface GraphView {
  /** 表示する時刻の範囲。 */
  t0: number;
  t1: number;
  /** 表示する値の範囲。 */
  v0: number;
  v1: number;
  width: number;
  height: number;
}

export function timeToX(t: number, view: Pick<GraphView, "t0" | "t1" | "width">): number {
  const span = view.t1 - view.t0 || 1;
  return ((t - view.t0) / span) * view.width;
}

export function xToTime(x: number, view: Pick<GraphView, "t0" | "t1" | "width">): number {
  const span = view.t1 - view.t0;
  return view.t0 + (x / (view.width || 1)) * span;
}

export function valueToY(v: number, view: GraphView): number {
  const span = view.v1 - view.v0 || 1;
  return view.height - ((v - view.v0) / span) * view.height;
}

export function yToValue(y: number, view: GraphView): number {
  const span = view.v1 - view.v0;
  return view.v0 + ((view.height - y) / (view.height || 1)) * span;
}

/**
 * カーブの SVG のパス。ベジェの区間は (時刻, 値) の平面の 3 次ベジェなので、SVG の `C` がそのまま同じ曲線になる。
 * ステップは終点の時刻まで始点の値 (`inverse_stepped` は始点の直後から終点の値)。
 * 最初のキーより前と最後のキーより後は、端の値の水平線を表示の範囲の端まで引く。
 */
export function curvePath(curve: Curve, view: GraphView): string {
  const keys = curveKeys(curve);
  const first = keys[0];
  if (!first) return "";
  const x = (t: number) => fmt(timeToX(t, view));
  const y = (v: number) => fmt(valueToY(v, view));
  const parts = [`M${x(Math.min(view.t0, first.t))},${y(first.v)}`, `L${x(first.t)},${y(first.v)}`];
  let prev = first;
  for (const segment of curve.segments.slice(1)) {
    const to = endPoint(segment);
    switch (segment.type) {
      case "linear":
        parts.push(`L${x(to.t)},${y(to.v)}`);
        break;
      case "bezier":
        parts.push(
          `C${x(segment.c1.t)},${y(segment.c1.v)} ${x(segment.c2.t)},${y(segment.c2.v)} ${x(to.t)},${y(to.v)}`,
        );
        break;
      case "stepped":
        parts.push(`L${x(to.t)},${y(prev.v)}`, `L${x(to.t)},${y(to.v)}`);
        break;
      case "inverse_stepped":
        parts.push(`L${x(prev.t)},${y(to.v)}`, `L${x(to.t)},${y(to.v)}`);
        break;
      case "start":
        break;
    }
    prev = to;
  }
  parts.push(`L${x(Math.max(view.t1, prev.t))},${y(prev.v)}`);
  return parts.join(" ");
}

function fmt(n: number): string {
  return String(Math.round(n * 100) / 100);
}

/** 値の表示範囲: パラメータなら値域、それ以外 (不透明度・エフェクトの強さ) は 0〜1。キーがはみ出せば広げる。 */
export function valueRange(
  curve: Curve | null,
  parameter: Pick<Parameter, "min" | "max"> | null,
): [number, number] {
  let [lo, hi] = parameter ? [parameter.min, parameter.max] : [0, 1];
  for (const segment of curve?.segments ?? []) {
    const points =
      segment.type === "bezier" ? [segment.c1, segment.c2, segment.to] : [endPoint(segment)];
    for (const p of points) {
      lo = Math.min(lo, p.v);
      hi = Math.max(hi, p.v);
    }
  }
  if (hi - lo < 1e-6) return [lo - 0.5, hi + 0.5];
  return [lo, hi];
}

/** 目盛りの間隔 (秒): 幅 `width` px に `duration` 秒を並べ、目盛りの間を 60px 以上あける。 */
export function tickStep(duration: number, width: number): number {
  const minPx = 60;
  const candidates = [1 / FPS, 0.1, 0.25, 0.5, 1, 2, 5, 10, 30, 60];
  for (const step of candidates) {
    if ((step / (duration || 1)) * width >= minPx) return step;
  }
  return 60;
}

// --- トラック ---

/** タイムラインの 1 行。 */
export interface Track {
  target: CurveTarget;
  label: string;
  curve: Curve | null;
  parameter: Parameter | null;
}

/**
 * 行の一覧: モデルのすべてのパラメータ (モデルの順) と、パラメータ以外を対象にするカーブ
 * (パーツの不透明度・エフェクトの強さ)、モデルにないパラメータのカーブ (最後)。
 */
export function tracks(motion: Motion | null, parameters: Parameter[]): Track[] {
  const rows: Track[] = parameters.map((p) => {
    const target: CurveTarget = { kind: "parameter", id: p.id };
    return {
      target,
      label: p.name && p.name !== p.id ? `${p.name} (${p.id})` : p.id,
      curve: motion ? findCurve(motion, target) : null,
      parameter: p,
    };
  });
  const known = new Set(parameters.map((p) => p.id));
  for (const curve of motion?.curves ?? []) {
    if (curve.target.kind === "parameter" && known.has(curve.target.id)) continue;
    const label =
      curve.target.kind === "parameter"
        ? `${curve.target.id} (missing)`
        : curve.target.kind === "part_opacity"
          ? `Opacity: ${curve.target.id}`
          : `Effect: ${curve.target.id}`;
    rows.push({ target: curve.target, label, curve, parameter: null });
  }
  return rows;
}

// --- コマンド ---

export function newMotion(name: string): Motion {
  return {
    name,
    duration: DEFAULT_DURATION,
    looping: false,
    fade_in: 0,
    fade_out: 0,
    curves: [],
    events: [],
  };
}

/** 既存の名前 (大文字小文字を区別しない) と重ならない `base`、`base_2`、… */
export function uniqueName(base: string, existing: string[]): string {
  const taken = new Set(existing.map((n) => n.toLowerCase()));
  if (!taken.has(base.toLowerCase())) return base;
  for (let n = 2; ; n++) {
    const name = `${base}_${n}`;
    if (!taken.has(name.toLowerCase())) return name;
  }
}

/**
 * モーション・表情の名前として使えるか (ファイル名になる。`[A-Za-z0-9_.-]`、`.` で始まらない、128 バイト以下。
 * crates/ugoe-editor-core/src/texture.rs の is_valid_texture_name と同じ規則)。
 */
export function isValidName(name: string): boolean {
  return /^[A-Za-z0-9_.-]{1,128}$/.test(name) && !name.startsWith(".");
}

export function keyCommand(motion: string, target: CurveTarget, t: number, v: number): Command {
  return { op: "set_motion_key", motion, target, point: { t, v } };
}

/**
 * 今のスライダーの値でキーを打つコマンド。`only` を与えればその対象だけ、なければ値が変わった
 * パラメータ (カーブがあればその時刻のカーブの値、なければ既定値と違うもの) だけ。
 * 複数なら 1 つの `batch` (Undo の 1 単位) にする。打つものがなければ null。
 */
export function keyValuesCommand(
  motion: Motion,
  parameters: Parameter[],
  values: Record<string, number>,
  t: number,
  sampled: Record<string, number>,
  only: string[] | null = null,
): Command | null {
  const commands: Command[] = [];
  for (const p of parameters) {
    const value = values[p.id];
    if (value === undefined) continue;
    if (only) {
      if (!only.includes(p.id)) continue;
    } else {
      const target: CurveTarget = { kind: "parameter", id: p.id };
      const base = findCurve(motion, target) ? sampled[p.id] : p.default;
      if (base !== undefined && Math.abs(base - value) <= tolerance(p)) continue;
    }
    commands.push(keyCommand(motion.name, { kind: "parameter", id: p.id }, t, value));
  }
  if (commands.length === 0) return null;
  if (commands.length === 1) return commands[0] ?? null;
  return { op: "batch", label: `Key ${commands.length} parameters`, commands };
}

/** 値が同じとみなす差 (値域の 1e-4 倍)。 */
function tolerance(p: Pick<Parameter, "min" | "max">): number {
  return Math.max(1e-6, (p.max - p.min) * 1e-4);
}

/** 区間の種類を変えるコマンドの `segment`。ベジェは今の取っ手を保つ (なければ既定の取っ手)。 */
export function segmentShape(kind: SegmentKind): SegmentShape {
  return kind === "bezier" ? { type: "bezier" } : { type: kind };
}

/**
 * 取っ手を区間 `[p0.t, to.t]` の中に収める (時刻だけ。値はそのまま)。
 */
export function clampHandle(c: Point, p0: Point, to: Point): Point {
  return { t: Math.min(Math.max(c.t, p0.t), to.t), v: c.v };
}

/**
 * キーを時刻 `t` へ動かせるか (隣のキーを越えない。同じ時刻にも重ねない)。
 */
export function canMoveKey(curve: Curve, from: number, to: number): boolean {
  const keys = curveKeys(curve);
  const i = keys.findIndex((k) => k.t === from);
  if (i < 0) return false;
  const prev = keys[i - 1];
  const next = keys[i + 1];
  return (!prev || prev.t < to) && (!next || to < next.t);
}

/** 区間の始点と終点 (時刻 `t` のキーで終わる区間)。 */
export function segmentEnds(curve: Curve, t: number): [Point, Point] | null {
  const keys = curveKeys(curve);
  const i = keys.findIndex((k) => k.t === t);
  const p0 = keys[i - 1];
  const to = keys[i];
  return i > 0 && p0 && to ? [p0, to] : null;
}
