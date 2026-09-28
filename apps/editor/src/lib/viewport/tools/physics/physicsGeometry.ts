// 物理の表示 (docs/editor.md#物理の画面での操作) の、DOM や wasm に依存しない幾何。physicsGeometry.test.ts で検査する。
//
// デフォーマの写像は関数で受け取る (実際は WebPuppet.deformer_point / deformer_jacobian)。
// 座標はキャンバス座標 (取り付け先が null なら恒等写像)。
import type { ColliderShape, PhysicsCollider, PhysicsRig, Vec2 } from "../../../ipc";
import type { Point } from "../geometry";

/** デフォーマ `deformer` の局所座標の点 → キャンバス座標 (`null` はキャンバスそのもの)。写せなければ null。 */
export type MapPoint = (deformer: string | null, p: Point) => Point | null;
/** デフォーマのワールド変換の、局所座標の点でのヤコビ行列 `[a, b, c, d]` (列優先)。 */
export type Jacobian = (
  deformer: string | null,
  p: Point,
) => [number, number, number, number] | null;

/** キャンバス座標に置いた衝突体の形状。 */
export type WorldShape =
  | { type: "circle"; center: Point; radius: number }
  | { type: "capsule"; a: Point; b: Point; radius: number };

/** 衝突体の形状をキャンバス座標に置く (位置だけを写し、半径はピクセルのまま。ugoe_physics と同じ)。 */
export function worldShape(collider: PhysicsCollider, map: MapPoint): WorldShape | null {
  const attach = collider.attach ?? null;
  const shape = collider.shape;
  if (shape.type === "circle") {
    const center = map(attach, shape.center);
    return center ? { type: "circle", center, radius: shape.radius } : null;
  }
  const a = map(attach, shape.a);
  const b = map(attach, shape.b);
  return a && b ? { type: "capsule", a, b, radius: shape.radius } : null;
}

/**
 * リグの根元を取り付けるデフォーマ: 出力の並びで最初の回転デフォーマの出力先、なければ重力の基準
 * (docs/spec/physics.md#物理空間と根元の取り付け先)。なければ null (キャンバス)。
 */
export function anchorOf(rig: PhysicsRig): string | null {
  for (const output of rig.outputs) {
    const t = output.target;
    if (t.type === "deformer_angle" || t.type === "deformer_offset") return t.deformer;
  }
  return rig.frame ?? null;
}

/**
 * 粒子の初期位置のキャンバス座標。根元は `F_A(particles[0].position)`、ほかの粒子は根元からの相対位置
 * (`position - particles[0].position`) を足した位置。プレビューしていないときの表示に使う。
 */
export function restParticles(rig: PhysicsRig, map: MapPoint): Point[] | null {
  const first = rig.particles[0];
  if (!first) return [];
  const root = map(anchorOf(rig), first.position);
  if (!root) return null;
  return rig.particles.map((p) => [
    root[0] + p.position[0] - first.position[0],
    root[1] + p.position[1] - first.position[1],
  ]);
}

/** 平坦な配列 `[x0, y0, ...]` を点の配列にする。 */
export function toPoints(flat: ArrayLike<number>): Point[] {
  const points: Point[] = [];
  for (let i = 0; i + 1 < flat.length; i += 2) points.push([flat[i] ?? 0, flat[i + 1] ?? 0]);
  return points;
}

const MAX_ITERATIONS = 32;
/** 逆写像の残差の許容 (キャンバスのピクセル)。 */
const TOLERANCE = 1e-3;

/**
 * キャンバス座標 `target` を、デフォーマ `deformer` の局所座標に写す (`map` の逆)。
 * ニュートン法 (初期値 `guess`、残差が減らなければ歩幅を半分にする) で解く。解けなければ null。
 */
export function canvasToLocal(
  map: MapPoint,
  jacobian: Jacobian,
  deformer: string | null,
  target: Point,
  guess: Point,
): Point | null {
  if (deformer === null) return target;
  let p: Point = guess;
  let q = map(deformer, p);
  if (!q) return null;
  let r: Point = [q[0] - target[0], q[1] - target[1]];
  for (let i = 0; i < MAX_ITERATIONS; i++) {
    if (Math.hypot(r[0], r[1]) <= TOLERANCE) return p;
    const j = jacobian(deformer, p);
    if (!j) return null;
    const [a, b, c, d] = j;
    const det = a * d - c * b;
    if (!Number.isFinite(det) || Math.abs(det) < 1e-12) return null;
    // J = [[a, c], [b, d]] (列優先)。step = J⁻¹ r。
    const step: Point = [(d * r[0] - c * r[1]) / det, (-b * r[0] + a * r[1]) / det];
    let t = 1;
    for (;;) {
      const next: Point = [p[0] - step[0] * t, p[1] - step[1] * t];
      q = map(deformer, next);
      if (q) {
        const nr: Point = [q[0] - target[0], q[1] - target[1]];
        if (Math.hypot(nr[0], nr[1]) < Math.hypot(r[0], r[1])) {
          p = next;
          r = nr;
          break;
        }
      }
      t *= 0.5;
      if (t < 1e-4) return Math.hypot(r[0], r[1]) <= TOLERANCE ? p : null;
    }
  }
  return Math.hypot(r[0], r[1]) <= TOLERANCE ? p : null;
}

/** 衝突体のつまみ: 円の中心 / カプセルの本体 (両端を一緒に動かす)・端 a・端 b。 */
export type ColliderHandle = "body" | "a" | "b";

/**
 * 衝突体のつまみ `handle` をキャンバス座標で `delta` だけ動かした形状 (局所座標)。
 * 動かした点をキャンバス座標で求め、取り付け先の局所座標に写し直す。写せなければ null。
 */
export function dragShape(
  collider: PhysicsCollider,
  handle: ColliderHandle,
  delta: Point,
  map: MapPoint,
  jacobian: Jacobian,
): ColliderShape | null {
  const attach = collider.attach ?? null;
  const move = (local: Vec2): Vec2 | null => {
    const world = map(attach, local);
    if (!world) return null;
    return canvasToLocal(map, jacobian, attach, [world[0] + delta[0], world[1] + delta[1]], local);
  };
  const shape = collider.shape;
  if (shape.type === "circle") {
    const center = move(shape.center);
    return center ? { ...shape, center } : null;
  }
  const a = handle === "b" ? shape.a : move(shape.a);
  const b = handle === "a" ? shape.b : move(shape.b);
  return a && b ? { ...shape, a, b } : null;
}

/** 点 `p` と線分 `a`–`b` の距離。 */
export function segmentDistance(p: Point, a: Point, b: Point): number {
  const ab: Point = [b[0] - a[0], b[1] - a[1]];
  const len2 = ab[0] * ab[0] + ab[1] * ab[1];
  const t =
    len2 > 0 ? Math.min(1, Math.max(0, ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / len2)) : 0;
  return Math.hypot(p[0] - (a[0] + ab[0] * t), p[1] - (a[1] + ab[1] * t));
}

/** カプセルの輪郭の SVG パス (線分の両側の直線と両端の半円)。座標は描く空間 (CSS ピクセル)。 */
export function capsulePath(a: Point, b: Point, radius: number): string {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const len = Math.hypot(dx, dy);
  if (len < 1e-9) {
    return `M${a[0] - radius},${a[1]}a${radius},${radius} 0 1,0 ${2 * radius},0a${radius},${radius} 0 1,0 ${-2 * radius},0Z`;
  }
  // 線分の法線 (長さ radius)。
  const nx = (-dy / len) * radius;
  const ny = (dx / len) * radius;
  const p1 = [a[0] + nx, a[1] + ny];
  const p2 = [b[0] + nx, b[1] + ny];
  const p3 = [b[0] - nx, b[1] - ny];
  const p4 = [a[0] - nx, a[1] - ny];
  return `M${p1[0]},${p1[1]}L${p2[0]},${p2[1]}A${radius},${radius} 0 0,0 ${p3[0]},${p3[1]}L${p4[0]},${p4[1]}A${radius},${radius} 0 0,0 ${p1[0]},${p1[1]}Z`;
}

/** 取り付け先の局所座標での、逆写像の初期値 (ワープは格子の中心、回転デフォーマは原点)。 */
export type LocalGuess = (deformer: string | null) => Point;

/**
 * 衝突体を別の取り付け先 `attach` に付け替える。今の見た目の位置 (キャンバス座標) を保つように、
 * 形状の位置を新しい取り付け先の局所座標に写し直す。写せなければ null。
 */
export function reattach(
  collider: PhysicsCollider,
  attach: string | null,
  map: MapPoint,
  jacobian: Jacobian,
  guess: LocalGuess,
): PhysicsCollider | null {
  const world = worldShape(collider, map);
  if (!world) return null;
  const shape = localShape(world, attach, map, jacobian, guess);
  if (!shape) return null;
  const { attach: _old, ...rest } = collider;
  return { ...rest, ...(attach ? { attach } : {}), shape };
}

/**
 * 衝突体の形状の種類を変える (円 ↔ カプセル)。キャンバス座標で、円はカプセルの中点に、
 * カプセルは円の中心を通る横向き (幅は半径の 2 倍) にする。
 */
export function changeShapeKind(
  collider: PhysicsCollider,
  kind: "circle" | "capsule",
  map: MapPoint,
  jacobian: Jacobian,
  guess: LocalGuess,
): ColliderShape | null {
  const world = worldShape(collider, map);
  if (!world || world.type === kind) return world ? collider.shape : null;
  const next: WorldShape =
    world.type === "circle"
      ? {
          type: "capsule",
          a: [world.center[0] - world.radius, world.center[1]],
          b: [world.center[0] + world.radius, world.center[1]],
          radius: world.radius,
        }
      : {
          type: "circle",
          center: [(world.a[0] + world.b[0]) / 2, (world.a[1] + world.b[1]) / 2],
          radius: world.radius,
        };
  return localShape(next, collider.attach ?? null, map, jacobian, guess);
}

function localShape(
  world: WorldShape,
  attach: string | null,
  map: MapPoint,
  jacobian: Jacobian,
  guess: LocalGuess,
): ColliderShape | null {
  const local = (p: Point) => canvasToLocal(map, jacobian, attach, p, guess(attach));
  if (world.type === "circle") {
    const center = local(world.center);
    return center ? { type: "circle", center, radius: world.radius } : null;
  }
  const a = local(world.a);
  const b = local(world.b);
  return a && b ? { type: "capsule", a, b, radius: world.radius } : null;
}
