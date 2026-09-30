// 評価: パラメータの値から、各メッシュの変形後の頂点・不透明度・描画順などを求める
// (spec/evaluation.md)。
//
// 計算は倍精度で行う (spec/evaluation.md#numbers-and-determinism は単精度以上を求め、
// 結果が許容誤差に収まればよいとしている)。足し合わせの順は仕様の順に従う。
// 動的な物理のオフセットとパーツの不透明度の上書きは扱わない (例のファイルでは使わない)。

import type * as M from "./types.ts";

type Vec2 = M.Vec2;

/** パラメータ ID → 現在の値。 */
export type ParameterValues = Map<string, number>;

/** 重みの並び: `[キーフォームの添字, 重み]` (spec/evaluation.md#keyform-interpolation)。 */
export type Weights = [number, number][];

export interface MeshResult {
  id: string;
  positions: Vec2[];
  opacity: number;
  draw_order: number;
  multiply_color: M.Rgb;
  screen_color: M.Rgb;
  mask_strength: number;
  visible: boolean;
}

export interface EvaluationResult {
  meshes: MeshResult[];
  /** 後ろから前へ並べたメッシュの添字 (spec/evaluation.md#render-order)。 */
  render_order: number[];
}

const clamp = (x: number, lo: number, hi: number): number => Math.min(Math.max(x, lo), hi);

// ---- パラメータ (spec/evaluation.md#parameter-values) ----

/** 既定値から始めて、与えられた値を範囲に切り詰めて書き込む。知らない ID は無視する。 */
export function parameterValues(model: M.Model, set: Record<string, number> = {}): ParameterValues {
  const values: ParameterValues = new Map();
  for (const p of model.parameters) values.set(p.id, p.default);
  for (const [id, value] of Object.entries(set)) {
    const p = model.parameters.find((q) => q.id === id);
    if (!p || !Number.isFinite(value)) continue;
    values.set(id, clamp(value, p.min, p.max));
  }
  return values;
}

// ---- 重み (spec/evaluation.md#keyform-interpolation) ----

/** 軸の上の位置 `(seg, t)` (spec/evaluation.md#locating-a-value-on-an-axis)。 */
export function locate(keys: number[], v: number): [number, number] {
  const n = keys.length;
  if (n === 1 || v <= keys[0]) return [0, 0];
  if (v >= keys[n - 1]) return [n - 1, 0];
  let seg = 0;
  while (!(keys[seg] <= v && v < keys[seg + 1])) seg++;
  return [seg, (v - keys[seg]) / (keys[seg + 1] - keys[seg])];
}

/** キー `i` の接線の係数 `[j, c_0, c_1, c_2]` (spec/evaluation.md#smooth-weights)。 */
function tangent(keys: number[], i: number): [number, number, number, number] {
  const n = keys.length;
  const j = clamp(i - 1, 0, n - 3);
  const h0 = keys[j + 1] - keys[j];
  const h1 = keys[j + 2] - keys[j + 1];
  const s = h0 + h1;
  let a: number;
  let b: number;
  if (i === j) {
    a = (2 * h0 + h1) / s;
    b = -h0 / s;
  } else if (i === j + 1) {
    a = h1 / s;
    b = h0 / s;
  } else {
    a = -h1 / s;
    b = (2 * h1 + h0) / s;
  }
  return [j, -a / h0, a / h0 - b / h1, b / h1];
}

/** 滑らかな軸の、位置 `(seg, t)` (`0 < t < 1`) での係数 `[base, A]`。 */
function smoothAxis(keys: number[], seg: number, t: number): [number, number[]] {
  const n = keys.length;
  const h = keys[seg + 1] - keys[seg];
  const t2 = t * t;
  const t3 = t2 * t;
  const h00 = 2 * t3 - 3 * t2 + 1;
  const h10 = t3 - 2 * t2 + t;
  const h01 = -2 * t3 + 3 * t2;
  const h11 = t3 - t2;
  const base = Math.max(seg - 1, 0);
  const A = [0, 0, 0, 0];
  A[seg - base] += h00;
  A[seg + 1 - base] += h01;
  for (const [key, basis] of [
    [seg, h10],
    [seg + 1, h11],
  ]) {
    const [j, ...c] = tangent(keys, key);
    for (let k = 0; k < 3; k++) A[j + k - base] += h * basis * c[k];
  }
  return [base, A.slice(0, Math.min(4, n - base))];
}

/**
 * グリッドの重み。`smooth` が真なら、3 つ以上のキーを持つ軸を滑らかに補間する
 * (spec/evaluation.md#linear-weights, #smooth-weights)。
 */
export function gridWeights(
  bindings: M.KeyformBinding[],
  values: ParameterValues,
  smooth: boolean,
): Weights {
  let W: Weights = [[0, 1]];
  let stride = 1;
  for (const binding of bindings) {
    const keys = binding.keys;
    const [seg, t] = locate(keys, values.get(binding.parameter) ?? 0);
    const m = W.length;
    if (smooth && keys.length >= 3 && t > 0) {
      const [base, A] = smoothAxis(keys, seg, t);
      for (let i = 0; i < m; i++) {
        const [index, w] = W[i];
        W[i] = [index + base * stride, w * A[0]];
        for (let c = 1; c < A.length; c++) W.push([index + (base + c) * stride, w * A[c]]);
      }
    } else {
      for (let i = 0; i < m; i++) {
        const [index, w] = W[i];
        W[i] = [index + seg * stride, w * (1 - t)];
        if (t > 0) W.push([index + (seg + 1) * stride, w * t]);
      }
    }
    stride *= keys.length;
  }
  W = W.filter(([, w]) => w !== 0 && !Number.isNaN(w));
  return W;
}

const sameWeights = (a: Weights, b: Weights): boolean =>
  a.length === b.length && a.every(([i, w], k) => i === b[k][0] && w === b[k][1]);

// ---- キーフォームの補間 (spec/evaluation.md#shape-fields-and-other-fields) ----

/**
 * キーフォームを数の並びにする方法: 形のフィールド (滑らかな重みで補間) と、
 * ほかのフィールド (線形の重みで補間)。
 */
interface Layout<T> {
  shape(k: T): number[];
  linear(k: T): number[];
}

function weightedSum<T>(keyforms: T[], W: Weights, flat: (k: T) => number[]): number[] {
  let out: number[] | null = null;
  for (const [index, w] of W) {
    const xs = flat(keyforms[index]);
    if (out === null) out = xs.map((x) => w * x);
    else for (let i = 0; i < xs.length; i++) out[i] += w * xs[i];
  }
  // 重みが空になることは 0 の重みを除いたときだけ起こる。そのときの値は 0。
  return out ?? flat(keyforms[0]).map(() => 0);
}

interface Interpolated<T> {
  shape: number[];
  linear: number[];
  /** ブール値のフィールドを選ぶための、通常のグリッドの線形の重み。 */
  linearWeights: Weights;
  keyforms: T[];
}

/**
 * 通常のグリッドを補間し、ブレンドシェイプの差分を足す (spec/evaluation.md#blend-shapes)。
 * `allowSmooth` が偽ならいつも線形 (パーツとグルー)。
 */
function interpolate<T>(
  grid: M.KeyformGrid<T>,
  layout: Layout<T>,
  values: ParameterValues,
  rest: ParameterValues,
  allowSmooth = true,
): Interpolated<T> {
  const smooth = allowSmooth && grid.interpolation === "smooth";
  const linW = gridWeights(grid.bindings, values, false);
  const shapeW = smooth ? gridWeights(grid.bindings, values, true) : linW;
  const shape = weightedSum(grid.keyforms, shapeW, layout.shape);
  const linear = weightedSum(grid.keyforms, linW, layout.linear);
  for (const b of grid.blend_shapes) {
    const bSmooth = allowSmooth && b.interpolation === "smooth";
    const curLin = gridWeights(b.bindings, values, false);
    const curShape = bSmooth ? gridWeights(b.bindings, values, true) : curLin;
    const restLin = gridWeights(b.bindings, rest, false);
    const restShape = bSmooth ? gridWeights(b.bindings, rest, true) : restLin;
    if (sameWeights(curShape, restShape) && sameWeights(curLin, restLin)) continue;
    const addDelta = (acc: number[], cur: Weights, restW: Weights, flat: (k: T) => number[]) => {
      const now = weightedSum(b.keyforms, cur, flat);
      const then = weightedSum(b.keyforms, restW, flat);
      for (let i = 0; i < acc.length; i++) acc[i] += now[i] - then[i];
    };
    addDelta(shape, curShape, restShape, layout.shape);
    addDelta(linear, curLin, restLin, layout.linear);
  }
  return { shape, linear, linearWeights: linW, keyforms: grid.keyforms };
}

/** 線形の重みが最大のキーフォーム (同じなら添字の小さいもの) (spec/evaluation.md#boolean-fields)。 */
function dominant<T>(ip: Interpolated<T>): T {
  let best = ip.linearWeights[0];
  for (const e of ip.linearWeights) {
    if (e[1] > best[1] || (e[1] === best[1] && e[0] < best[0])) best = e;
  }
  return ip.keyforms[best ? best[0] : 0];
}

const flatPoints = (ps: Vec2[]): number[] => ps.flat();
const toPoints = (xs: number[]): Vec2[] => {
  const out: Vec2[] = [];
  for (let i = 0; i < xs.length; i += 2) out.push([xs[i], xs[i + 1]]);
  return out;
};

const meshLayout: Layout<M.MeshKeyform> = {
  shape: (k) => flatPoints(k.positions),
  linear: (k) => [
    k.opacity,
    k.draw_order,
    k.multiply_color.r,
    k.multiply_color.g,
    k.multiply_color.b,
    k.screen_color.r,
    k.screen_color.g,
    k.screen_color.b,
    k.mask_strength,
  ],
};

const warpLayout: Layout<M.WarpKeyform> = {
  shape: (k) => flatPoints(k.points),
  linear: (k) => [k.opacity],
};

const rotationLayout: Layout<M.RotationKeyform> = {
  shape: (k) => [k.origin[0], k.origin[1], k.angle, k.scale],
  linear: (k) => [k.opacity],
};

const partLayout: Layout<M.PartKeyform> = { shape: () => [], linear: (k) => [k.opacity] };
const glueLayout: Layout<M.GlueKeyform> = { shape: () => [], linear: (k) => [k.intensity] };

// ---- 変換 (spec/evaluation.md#world-transforms) ----

export type World =
  | { kind: "identity" }
  /** `F(p) = t + m · p`。`m = [m00, m01, m10, m11]` (行ごと)。 */
  | { kind: "affine"; m: [number, number, number, number]; t: Vec2 }
  | { kind: "warp"; cols: number; rows: number; P: Vec2[] };

const IDENTITY: World = { kind: "identity" };

/** 格子の中の双線形の写像 (spec/evaluation.md#warp-deformers)。 */
function bilinear(w: { cols: number; rows: number; P: Vec2[] }, u: number, v: number): Vec2 {
  const { cols, rows, P } = w;
  const gx = u * cols;
  const gy = v * rows;
  const i = clamp(Math.floor(gx), 0, cols - 1);
  const j = clamp(Math.floor(gy), 0, rows - 1);
  const fx = gx - i;
  const fy = gy - j;
  const at = (x: number, y: number): Vec2 => P[x + (cols + 1) * y];
  const lerp = (a: Vec2, b: Vec2, t: number): Vec2 => [
    a[0] + (b[0] - a[0]) * t,
    a[1] + (b[1] - a[1]) * t,
  ];
  return lerp(lerp(at(i, j), at(i + 1, j), fx), lerp(at(i, j + 1), at(i + 1, j + 1), fx), fy);
}

export function apply(F: World, p: Vec2): Vec2 {
  switch (F.kind) {
    case "identity":
      return [p[0], p[1]];
    case "affine": {
      const [a, b, c, d] = F.m;
      return [F.t[0] + a * p[0] + b * p[1], F.t[1] + c * p[0] + d * p[1]];
    }
    case "warp": {
      const { cols, rows, P } = F;
      const [u, v] = p;
      const cu = clamp(u, 0, 1);
      const cv = clamp(v, 0, 1);
      const B = bilinear(F, cu, cv);
      const du = u - cu;
      const dv = v - cv;
      if (du === 0 && dv === 0) return B;
      const C00 = P[0];
      const C10 = P[cols];
      const C01 = P[(cols + 1) * rows];
      const C11 = P[cols + (cols + 1) * rows];
      const Au: Vec2 = [
        (C10[0] - C00[0] + (C11[0] - C01[0])) * 0.5,
        (C10[1] - C00[1] + (C11[1] - C01[1])) * 0.5,
      ];
      const Av: Vec2 = [
        (C01[0] - C00[0] + (C11[0] - C10[0])) * 0.5,
        (C01[1] - C00[1] + (C11[1] - C10[1])) * 0.5,
      ];
      return [B[0] + Au[0] * du + Av[0] * dv, B[1] + Au[1] * du + Av[1] * dv];
    }
  }
}

const DEG = Math.PI / 180;

/** `R(θ) · S` を行ごとに。 */
function rotScale(thetaDeg: number, sx: number, sy: number): [number, number, number, number] {
  const c = Math.cos(thetaDeg * DEG);
  const s = Math.sin(thetaDeg * DEG);
  return [c * sx, -s * sy, s * sx, c * sy];
}

/** 回転デフォーマのワールド変換 (spec/evaluation.md#rotation-deformers)。 */
function rotationWorld(parent: World, k: RotationValues): World {
  const sx = k.scale * (k.reflect_x ? -1 : 1);
  const sy = k.scale * (k.reflect_y ? -1 : 1);
  const alpha = k.angle;
  switch (parent.kind) {
    case "identity":
      return { kind: "affine", m: rotScale(alpha, sx, sy), t: k.origin };
    case "affine": {
      const [a, b, c, d] = parent.m;
      const [e, f, g, h] = rotScale(alpha, sx, sy);
      return {
        kind: "affine",
        m: [a * e + b * g, a * f + b * h, c * e + d * g, c * f + d * h],
        t: apply(parent, k.origin),
      };
    }
    case "warp": {
      const eps = 0.01;
      const o = apply(parent, k.origin);
      const tip = apply(parent, [k.origin[0], k.origin[1] - eps]);
      const up: Vec2 = [tip[0] - o[0], tip[1] - o[1]];
      // atan2(0, 0) は 0 (JS の Math.atan2(0, -0) は π なので、ここで決める)
      const delta = up[0] === 0 && up[1] === 0 ? 0 : Math.atan2(up[0], -up[1]) / DEG;
      return { kind: "affine", m: rotScale(alpha + delta, sx, sy), t: o };
    }
  }
}

interface RotationValues {
  origin: Vec2;
  angle: number;
  scale: number;
  reflect_x: boolean;
  reflect_y: boolean;
}

// ---- 評価 (spec/evaluation.md#order-of-evaluation) ----

/** 1 つのデフォーマの結果。 */
export interface DeformerState {
  world: World;
  opacity: number;
}

export function evaluate(model: M.Model, values: ParameterValues): EvaluationResult {
  const rest = parameterValues(model);

  // 1. パーツ: 親から子へ (spec/evaluation.md#parts)
  const partById = new Map(model.parts.map((p) => [p.id, p]));
  const partOpacity = new Map<string, number>();
  const partOpacityOf = (id: string): number => {
    const cached = partOpacity.get(id);
    if (cached !== undefined) return cached;
    const p = partById.get(id);
    if (!p) throw new Error(`unknown part ${id}`);
    const own = interpolate(p.keyforms, partLayout, values, rest, false).linear[0];
    const result = own * (p.parent === null ? 1 : partOpacityOf(p.parent));
    partOpacity.set(id, result);
    return result;
  };

  // 2. デフォーマ: 親から子へ (spec/evaluation.md#deformers)
  const deformerById = new Map(model.deformers.map((d) => [d.id, d]));
  const deformerState = new Map<string, DeformerState>();
  const deformerOf = (id: string): DeformerState => {
    const cached = deformerState.get(id);
    if (cached) return cached;
    const d = deformerById.get(id);
    if (!d) throw new Error(`unknown deformer ${id}`);
    const parent: DeformerState =
      d.parent === null ? { world: IDENTITY, opacity: 1 } : deformerOf(d.parent);
    let state: DeformerState;
    switch (d.kind.type) {
      case "warp": {
        const { cols, rows } = d.kind;
        const ip = interpolate(d.kind.keyforms, warpLayout, values, rest);
        const P = toPoints(ip.shape).map((q) => apply(parent.world, q));
        state = { world: { kind: "warp", cols, rows, P }, opacity: ip.linear[0] * parent.opacity };
        break;
      }
      case "rotation": {
        const ip = interpolate(d.kind.keyforms, rotationLayout, values, rest);
        const k = dominant(ip);
        const [ox, oy, angle, scale] = ip.shape;
        const world = rotationWorld(parent.world, {
          origin: [ox, oy],
          angle,
          scale,
          reflect_x: k.reflect_x,
          reflect_y: k.reflect_y,
        });
        state = { world, opacity: ip.linear[0] * parent.opacity };
        break;
      }
      case "unknown":
        // 親の変換をそのまま使い、自身の不透明度は 1 (spec/evaluation.md#unknown-kinds)
        state = { world: parent.world, opacity: parent.opacity };
        break;
    }
    deformerState.set(id, state);
    return state;
  };
  for (const d of model.deformers) deformerOf(d.id);

  // 3. メッシュ (spec/evaluation.md#meshes)
  const meshes: MeshResult[] = model.meshes.map((m) => {
    const ip = interpolate(m.keyforms, meshLayout, values, rest);
    const parent: DeformerState =
      m.parent === null ? { world: IDENTITY, opacity: 1 } : deformerOf(m.parent);
    const positions = toPoints(ip.shape).map((p) => apply(parent.world, p));
    const [own, draw_order, mr, mg, mb, sr, sg, sb, mask] = ip.linear;
    const opacity = own * partOpacityOf(m.part) * parent.opacity;
    return {
      id: m.id,
      positions,
      opacity,
      draw_order,
      multiply_color: { r: mr, g: mg, b: mb },
      screen_color: { r: sr, g: sg, b: sb },
      mask_strength: clamp(mask, 0, 1),
      visible: !m.mask_only && opacity > 0,
    };
  });

  // 4. グルー: 配列の順に (spec/evaluation.md#glues)
  const meshIndex = new Map(model.meshes.map((m, i) => [m.id, i]));
  for (const g of model.glues) {
    const intensity = interpolate(g.keyforms, glueLayout, values, rest, false).linear[0];
    const a = meshes[meshIndex.get(g.mesh_a) ?? -1];
    const b = meshes[meshIndex.get(g.mesh_b) ?? -1];
    for (const pair of g.pairs) {
      const pa = a.positions[pair.a];
      const pb = b.positions[pair.b];
      const target: Vec2 = [
        pa[0] + (pb[0] - pa[0]) * pair.weight,
        pa[1] + (pb[1] - pa[1]) * pair.weight,
      ];
      a.positions[pair.a] = [
        pa[0] + (target[0] - pa[0]) * intensity,
        pa[1] + (target[1] - pa[1]) * intensity,
      ];
      b.positions[pair.b] = [
        pb[0] + (target[0] - pb[0]) * intensity,
        pb[1] + (target[1] - pb[1]) * intensity,
      ];
    }
  }

  // 5. 描画順: 安定な並べ替え、totalOrder で比べる (spec/evaluation.md#render-order)
  const render_order = meshes.map((_, i) => i);
  render_order.sort((i, j) => totalOrder(meshes[i].draw_order, meshes[j].draw_order) || i - j);

  return { meshes, render_order };
}

/** IEEE 754 の totalOrder による比較 (負・0・正)。 */
export function totalOrder(a: number, b: number): number {
  const key = (x: number): bigint => {
    const view = new DataView(new ArrayBuffer(8));
    view.setFloat64(0, x);
    const bits = view.getBigUint64(0);
    const sign = 1n << 63n;
    return bits & sign ? ~bits & (2n ** 64n - 1n) : bits | sign;
  };
  const ka = key(a);
  const kb = key(b);
  return ka < kb ? -1 : ka > kb ? 1 : 0;
}
