// 描画: 評価の結果を CPU で画素に塗る小さなラスタライザ (spec/rendering.md)。
//
// 画素の中心 (x + 0.5, y + 0.5) で三角形の内外を決め (辺の上は左上の規則)、テクスチャ座標を線形に補間し、
// 乗算済みの値を双線形・clamp-to-edge で読む。色は倍精度の [0, 1] で持ち、描くたびに [0, 1] に切り詰める
// (spec/rendering.md#color-values の「範囲の限られた形式」を 8 ビットに量子化せずに真似る)。
// 速さは求めない (例の画像は数百 × 数百の画素まで)。

import type { EvaluationResult } from "./evaluate.ts";
import type * as M from "./types.ts";

export type Vec3 = [number, number, number];

/** 非乗算の RGBA 8 ビットの画像 (行ごと、左上から)。 */
export interface Image {
  width: number;
  height: number;
  rgba: ArrayLike<number>;
}

/** ホストが与える光源 (spec/rendering.md#lights)。`range` が無い点光源は減衰しない。 */
export interface Lights {
  ambient?: Vec3;
  directional?: { direction: Vec3; color: Vec3 }[];
  point?: { position: Vec3; color: Vec3; range?: number }[];
}

export interface RenderInput {
  model: M.Model;
  result: EvaluationResult;
  /** モデルの `textures` の順の画像。 */
  textures: Image[];
  /** テクスチャごとの法線マップ (無ければ `null`)。 */
  normalMaps: (Image | null)[];
  width: number;
  height: number;
  /** 省略: ホストは光源を与えない。 */
  lights?: Lights;
}

/**
 * 透明な黒で初期化した `width` × `height` の描画先に描き、乗算済みの RGBA ([0, 1]) を返す。
 * ビュー変換はキャンバス全体を描画先に引き伸ばすもの。
 */
export function render(input: RenderInput): Float64Array {
  const { model, result, width, height } = input;
  const sx = width / model.meta.canvas.width;
  const sy = height / model.meta.canvas.height;
  const out = new Float64Array(4 * width * height);
  const meshIndex = new Map(model.meshes.map((m, i) => [m.id, i]));
  const lighting = model.lighting;
  const lights = validLights(input.lights);

  // マスクの係数とカバレッジは、画素ごとの値を必要になったときに 1 度だけ求める (spec/rendering.md#masks)
  const factors = new Map<number, Float64Array | null>();
  const coverages = new Map<number, Float64Array>();

  const triangles = (i: number): Triangle[] => {
    const m = model.meshes[i];
    const pos = result.meshes[i].positions;
    const tris: Triangle[] = [];
    for (let t = 0; t + 2 < m.indices.length; t += 3) {
      const ix = [m.indices[t], m.indices[t + 1], m.indices[t + 2]];
      tris.push({
        canvas: ix.map((k) => pos[k]),
        target: ix.map((k) => [pos[k][0] * sx, pos[k][1] * sy]),
        uv: ix.map((k) => m.uvs[k]),
      });
    }
    return tris;
  };

  const coverageOf = (i: number): Float64Array => {
    const cached = coverages.get(i);
    if (cached) return cached;
    const m = model.meshes[i];
    const image = input.textures[m.texture];
    const rest = new Float64Array(width * height).fill(1);
    for (const tri of triangles(i)) {
      rasterize(tri.target, width, height, (x, y, l) => {
        const [u, v] = interpolateUv(tri.uv, l);
        rest[y * width + x] *= 1 - sample(image, u, v, true)[3];
      });
    }
    const k = factorOf(i);
    const c = new Float64Array(width * height);
    for (let p = 0; p < c.length; p++) c[p] = (1 - rest[p]) * (k ? k[p] : 1);
    coverages.set(i, c);
    return c;
  };

  const factorOf = (i: number): Float64Array | null => {
    if (factors.has(i)) return factors.get(i) ?? null;
    const m = model.meshes[i];
    if (m.masks.length === 0) {
      factors.set(i, null);
      return null;
    }
    const ids = (invert: boolean) => [
      ...new Set(m.masks.filter((r) => r.invert === invert).map((r) => r.mesh)),
    ];
    const lookup = (id: string) => {
      const j = meshIndex.get(id);
      if (j === undefined) throw new Error(`unknown mask mesh ${id}`);
      return coverageOf(j);
    };
    const P = ids(false).map(lookup);
    const N = ids(true).map(lookup);
    const s = result.meshes[i].mask_strength;
    const k = new Float64Array(width * height);
    for (let p = 0; p < k.length; p++) {
      let pos = 1;
      if (P.length > 0) {
        if (m.mask_mode === "intersection") pos = P.reduce((acc, c) => acc * c[p], 1);
        else pos = 1 - P.reduce((acc, c) => acc * (1 - c[p]), 1);
      }
      const neg = 1 - N.reduce((acc, c) => acc * (1 - c[p]), 1);
      k[p] = 1 - s * (1 - pos * (1 - neg));
    }
    factors.set(i, k);
    return k;
  };

  for (const i of result.render_order) {
    const m = model.meshes[i];
    const r = result.meshes[i];
    if (!r.visible) continue;
    const image = input.textures[m.texture];
    const normalMap = input.normalMaps[m.texture] ?? null;
    const lit = lighting && m.lit;
    const k = factorOf(i);
    for (const tri of triangles(i)) {
      if (m.culling && !facesFront(tri.canvas)) continue;
      const rotation = lit ? normalRotation(tri) : null;
      rasterize(tri.target, width, height, (x, y, l) => {
        const [u, v] = interpolateUv(tri.uv, l);
        const c = sample(image, u, v, true);
        // spec/rendering.md#mesh-color
        let rgb = [0, 1, 2].map((q) => c[q] * rgbAt(r.multiply_color, q));
        rgb = rgb.map((x, q) => {
          const S = rgbAt(r.screen_color, q);
          return x + S * c[3] - x * S;
        });
        if (rotation) {
          const n = normalAt(normalMap, u, v, rotation);
          const light = shade(n, [(x + 0.5) / sx, (y + 0.5) / sy], lights);
          rgb = rgb.map((x, q) => x * light[q]);
        }
        const f = r.opacity * (k ? k[y * width + x] : 1);
        const src = [rgb[0] * f, rgb[1] * f, rgb[2] * f, c[3] * f];
        blend(out, 4 * (y * width + x), src, m.blend);
      });
    }
  }
  return out;
}

interface Triangle {
  canvas: M.Vec2[];
  target: M.Vec2[];
  uv: M.Vec2[];
}

const rgbAt = (c: M.Rgb, q: number): number => (q === 0 ? c.r : q === 1 ? c.g : c.b);

const clamp01 = (x: number): number => Math.min(Math.max(x, 0), 1);

/** spec/rendering.md#blend-modes。書いた後に [0, 1] に切り詰める。 */
function blend(out: Float64Array, at: number, src: number[], mode: M.Mesh["blend"]) {
  const dst = [out[at], out[at + 1], out[at + 2], out[at + 3]];
  const inv = 1 - src[3];
  let res: number[];
  if (mode === "additive") res = [src[0] + dst[0], src[1] + dst[1], src[2] + dst[2], dst[3]];
  else if (mode === "multiply") {
    res = [0, 1, 2].map((q) => src[q] * dst[q] + dst[q] * inv).concat([dst[3]]);
  } else res = [0, 1, 2, 3].map((q) => src[q] + dst[q] * inv);
  for (let q = 0; q < 4; q++) out[at + q] = clamp01(res[q]);
}

// ---- ラスタライズ ----

const edge = (a: M.Vec2, b: M.Vec2, p: M.Vec2): number =>
  (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);

/**
 * 三角形が覆う画素ごとに `visit(x, y, 重心座標)` を呼ぶ。画素の中心で内外を決め、辺の上の中心は
 * 左上の規則で 1 つの三角形にだけ入れる (隣り合う三角形が同じ画素を 2 度塗らない)。
 */
export function rasterize(
  tri: M.Vec2[],
  width: number,
  height: number,
  visit: (x: number, y: number, l: [number, number, number]) => void,
) {
  let [a, b, c] = tri;
  let area = edge(a, b, c);
  if (area === 0 || !Number.isFinite(area)) return;
  // y が下向きの画面で時計回り (area > 0) にそろえる
  if (area < 0) {
    [b, c] = [c, b];
    area = -area;
  }
  const swapped = edge(tri[0], tri[1], tri[2]) < 0;
  // 左上の規則: 上の辺 (水平で、ほかの頂点より上) と左の辺 (下へ向かう辺。時計回りでは y が減る辺) を含める
  const topLeft = (p: M.Vec2, q: M.Vec2): boolean => (p[1] === q[1] && q[0] > p[0]) || q[1] < p[1];
  const edges: [M.Vec2, M.Vec2][] = [
    [b, c],
    [c, a],
    [a, b],
  ];
  const bias = edges.map(([p, q]) => topLeft(p, q));
  const xs = [a[0], b[0], c[0]];
  const ys = [a[1], b[1], c[1]];
  const x0 = Math.max(0, Math.floor(Math.min(...xs) - 0.5));
  const x1 = Math.min(width - 1, Math.ceil(Math.max(...xs) - 0.5));
  const y0 = Math.max(0, Math.floor(Math.min(...ys) - 0.5));
  const y1 = Math.min(height - 1, Math.ceil(Math.max(...ys) - 0.5));
  for (let y = y0; y <= y1; y++) {
    for (let x = x0; x <= x1; x++) {
      const p: M.Vec2 = [x + 0.5, y + 0.5];
      const w = edges.map(([e0, e1]) => edge(e0, e1, p));
      if (!w.every((wi, k) => wi > 0 || (wi === 0 && bias[k]))) continue;
      // w[0] は a の、w[1] は b の、w[2] は c の重み
      const la = w[0] / area;
      const lb = w[1] / area;
      const lc = w[2] / area;
      visit(x, y, swapped ? [la, lc, lb] : [la, lb, lc]);
    }
  }
}

function interpolateUv(uv: M.Vec2[], l: [number, number, number]): M.Vec2 {
  return [
    l[0] * uv[0][0] + l[1] * uv[1][0] + l[2] * uv[2][0],
    l[0] * uv[0][1] + l[1] * uv[1][1] + l[2] * uv[2][1],
  ];
}

/** spec/rendering.md#back-face-culling: キャンバス座標で反時計回り (値が負) なら表。 */
function facesFront(p: M.Vec2[]): boolean {
  return (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]) < 0;
}

// ---- テクスチャ (spec/rendering.md#textures) ----

/** 双線形・clamp-to-edge。`premultiply` なら各テクセルを乗算済みにしてから補間する。[0, 1] の RGBA。 */
export function sample(image: Image, u: number, v: number, premultiply: boolean): number[] {
  const { width, height, rgba } = image;
  const texel = (i: number, j: number): number[] => {
    const at =
      4 * (Math.min(Math.max(j, 0), height - 1) * width + Math.min(Math.max(i, 0), width - 1));
    const a = rgba[at + 3] / 255;
    const f = premultiply ? a : 1;
    return [(rgba[at] / 255) * f, (rgba[at + 1] / 255) * f, (rgba[at + 2] / 255) * f, a];
  };
  const x = u * width - 0.5;
  const y = v * height - 0.5;
  const i = Math.floor(x);
  const j = Math.floor(y);
  const fx = x - i;
  const fy = y - j;
  const t00 = texel(i, j);
  const t10 = texel(i + 1, j);
  const t01 = texel(i, j + 1);
  const t11 = texel(i + 1, j + 1);
  return [0, 1, 2, 3].map(
    (q) => (t00[q] * (1 - fx) + t10[q] * fx) * (1 - fy) + (t01[q] * (1 - fx) + t11[q] * fx) * fy,
  );
}

// ---- ライティング (spec/rendering.md#lighting) ----

interface Rotation {
  cos: number;
  sin: number;
  /** 三角形が鏡映されている (det J < 0) なら、法線の x を反転する。 */
  mirrored: boolean;
}

/** 三角形のテクスチャ座標からキャンバス座標への写像 J = [t b] と、その極分解の回転 (spec/rendering.md#normals)。 */
function normalRotation(tri: Triangle): Rotation {
  const [p0, p1, p2] = tri.canvas;
  const [q0, q1, q2] = tri.uv;
  const du1 = q1[0] - q0[0];
  const dv1 = q1[1] - q0[1];
  const du2 = q2[0] - q0[0];
  const dv2 = q2[1] - q0[1];
  const dx1 = p1[0] - p0[0];
  const dy1 = p1[1] - p0[1];
  const dx2 = p2[0] - p0[0];
  const dy2 = p2[1] - p0[1];
  const det = du1 * dv2 - du2 * dv1;
  if (det === 0) return { cos: 1, sin: 0, mirrored: false };
  // dP = t du + b dv を 2 つの辺について解く
  let t: M.Vec2 = [(dx1 * dv2 - dx2 * dv1) / det, (dy1 * dv2 - dy2 * dv1) / det];
  const b: M.Vec2 = [(du1 * dx2 - du2 * dx1) / det, (du1 * dy2 - du2 * dy1) / det];
  const mirrored = t[0] * b[1] - b[0] * t[1] < 0;
  if (mirrored) t = [-t[0], -t[1]];
  const r: M.Vec2 = [t[0] + b[1], t[1] - b[0]];
  const len = Math.hypot(r[0], r[1]);
  return len > 0 ? { cos: r[0] / len, sin: r[1] / len, mirrored } : { cos: 1, sin: 0, mirrored };
}

function normalAt(normalMap: Image | null, u: number, v: number, rot: Rotation): Vec3 {
  let n: Vec3 = [0, 0, 1];
  if (normalMap) {
    const c = sample(normalMap, u, v, false);
    n = [2 * c[0] - 1, 2 * c[1] - 1, 2 * c[2] - 1];
  }
  // 1. UV の向きへ (v は下向き)。3. 鏡映なら x を反転
  let nx = rot.mirrored ? -n[0] : n[0];
  const ny = -n[1];
  // 5. xy を回す。6. 正規化
  const x = rot.cos * nx - rot.sin * ny;
  const y = rot.sin * nx + rot.cos * ny;
  nx = x;
  const len = Math.hypot(nx, y, n[2]);
  return len > 0 ? [nx / len, y / len, n[2] / len] : [0, 0, 1];
}

interface ValidLights {
  ambient: Vec3;
  directional: { l: Vec3; color: Vec3 }[];
  point: { position: Vec3; color: Vec3; range: number }[];
}

function validLights(lights: Lights | undefined): ValidLights {
  const directional: ValidLights["directional"] = [];
  for (const d of lights?.directional ?? []) {
    const len = Math.hypot(...d.direction);
    if (!(len > 0) || !Number.isFinite(len)) continue;
    directional.push({ l: d.direction.map((x) => -x / len) as Vec3, color: d.color });
  }
  const point: ValidLights["point"] = [];
  for (const p of lights?.point ?? []) {
    const range = p.range ?? Number.POSITIVE_INFINITY;
    if (!(range > 0) || !p.position.every(Number.isFinite)) continue;
    point.push({ position: p.position, color: p.color, range });
  }
  return { ambient: lights?.ambient ?? [1, 1, 1], directional, point };
}

/** `light = ambient + Σ color × max(0, n·l) × attenuation`。`at` は画素の中心のキャンバス座標。 */
function shade(n: Vec3, at: M.Vec2, lights: ValidLights): Vec3 {
  const out: Vec3 = [...lights.ambient];
  const add = (l: Vec3, color: Vec3, attenuation: number) => {
    const d = Math.max(0, n[0] * l[0] + n[1] * l[1] + n[2] * l[2]) * attenuation;
    for (let q = 0; q < 3; q++) out[q] += color[q] * d;
  };
  for (const d of lights.directional) add(d.l, d.color, 1);
  for (const p of lights.point) {
    const v: Vec3 = [p.position[0] - at[0], p.position[1] - at[1], p.position[2]];
    const dist = Math.hypot(...v);
    const l: Vec3 = dist > 0 ? (v.map((x) => x / dist) as Vec3) : [0, 0, 1];
    const attenuation = p.range === Number.POSITIVE_INFINITY ? 1 : clamp01(1 - dist / p.range) ** 2;
    add(l, p.color, attenuation);
  }
  return out;
}
