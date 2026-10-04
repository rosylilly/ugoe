// デフォーマのギズモの幾何 (docs/editor.md#デフォーマの編集)。DOM や wasm に依存しない (gizmo.test.ts)。
// 座標系は呼び出し側がそろえる (ギズモは CSS ピクセルで描き、コマンドにはキャンバス座標を渡す)。
// Y は下向きなので、角度 (度) は画面上で時計回りが正 (ugoe_model の回転デフォーマと同じ)。

export type Point = [number, number];

/** 平坦な配列 [x0, y0, x1, y1, ...] を点の配列にする。 */
export function toPoints(flat: ArrayLike<number>): Point[] {
  const out: Point[] = [];
  for (let i = 0; i + 1 < flat.length; i += 2) out.push([flat[i] ?? 0, flat[i + 1] ?? 0]);
  return out;
}

/** 点の配列を平坦な配列にする (geometry.ts の nearestVertex などに渡す)。 */
export function flatten(points: readonly Point[]): number[] {
  return points.flatMap((p) => [p[0], p[1]]);
}

/** ワープの格子 (cols × rows セル、行優先で x が速い制御点) の辺。 */
export function gridEdges(cols: number, rows: number): [number, number][] {
  const at = (x: number, y: number) => y * (cols + 1) + x;
  const out: [number, number][] = [];
  for (let y = 0; y <= rows; y++) {
    for (let x = 0; x <= cols; x++) {
      if (x < cols) out.push([at(x, y), at(x + 1, y)]);
      if (y < rows) out.push([at(x, y), at(x, y + 1)]);
    }
  }
  return out;
}

/** 格子の外周の制御点の番号 (上の行 → 右の列 → 下の行 → 左の列の順に一周)。 */
export function gridOutline(cols: number, rows: number): number[] {
  const at = (x: number, y: number) => y * (cols + 1) + x;
  const out: number[] = [];
  for (let x = 0; x <= cols; x++) out.push(at(x, 0));
  for (let y = 1; y <= rows; y++) out.push(at(cols, y));
  for (let x = cols - 1; x >= 0; x--) out.push(at(x, rows));
  for (let y = rows - 1; y >= 1; y--) out.push(at(0, y));
  return out;
}

/** 選んだ点だけを `delta` だけずらす (元の配列は変えない)。 */
export function movePoints(
  points: readonly Point[],
  indices: readonly number[],
  delta: Point,
): Point[] {
  const moved = new Set(indices);
  return points.map((p, i) => (moved.has(i) ? [p[0] + delta[0], p[1] + delta[1]] : p));
}

/**
 * 回転デフォーマのワールド変換 (WebPuppet.rotation_frame): `world(p) = origin + x * p.x + y * p.y`。
 * x・y は局所座標の単位ベクトルの像 (回転・拡大率・反転を含む)。
 */
export interface RotationFrame {
  origin: Point;
  x: Point;
  y: Point;
}

export function rotationFrame(flat: ArrayLike<number> | null | undefined): RotationFrame | null {
  if (!flat || flat.length < 6) return null;
  const v = Array.from(flat);
  if (v.some((n) => !Number.isFinite(n))) return null;
  return {
    origin: [v[0] ?? 0, v[1] ?? 0],
    x: [v[2] ?? 0, v[3] ?? 0],
    y: [v[4] ?? 0, v[5] ?? 0],
  };
}

/** 点に一様な拡大と平行移動 (ビューの変換 `offset + scale * p`) を掛けた枠。 */
export function transformFrame(
  frame: RotationFrame,
  map: (p: Point) => Point,
  scale: number,
): RotationFrame {
  return {
    origin: map(frame.origin),
    x: [frame.x[0] * scale, frame.x[1] * scale],
    y: [frame.y[0] * scale, frame.y[1] * scale],
  };
}

function unit(v: Point, fallback: Point): Point {
  const n = Math.hypot(v[0], v[1]);
  return n > 1e-9 ? [v[0] / n, v[1] / n] : fallback;
}

/**
 * 回転デフォーマのギズモの取っ手: 原点、回転 (局所座標の上 = -y の向き)、拡大率 (局所座標の +x の向き)。
 * 取っ手は原点から `length` の位置に置く (拡大率によらず同じ長さ)。
 */
export function rotationHandles(
  frame: RotationFrame,
  length: number,
): { origin: Point; rotate: Point; scale: Point } {
  const [ox, oy] = frame.origin;
  const up = unit([-frame.y[0], -frame.y[1]], [0, -1]);
  const right = unit(frame.x, [1, 0]);
  return {
    origin: [ox, oy],
    rotate: [ox + up[0] * length, oy + up[1] * length],
    scale: [ox + right[0] * length, oy + right[1] * length],
  };
}

/** `center` から見た `from` → `to` の回転角 (度、(-180, 180]、画面上で時計回りが正)。 */
export function angleBetween(center: Point, from: Point, to: Point): number {
  const a = Math.atan2(from[1] - center[1], from[0] - center[0]);
  const b = Math.atan2(to[1] - center[1], to[0] - center[0]);
  let d = ((b - a) * 180) / Math.PI;
  while (d <= -180) d += 360;
  while (d > 180) d -= 360;
  return d;
}

/**
 * 回転の取っ手のドラッグで、前の位置 `from` から今の位置 `to` までの回転 (度) を `turn` に足す。1 回の動きごとに
 * 測って足すので、180 度を超えて回せる (始点からの角度だけで測ると ±180 度で反対向きに折り返す)。
 */
export function addTurn(turn: number, center: Point, from: Point, to: Point): number {
  return turn + angleBetween(center, from, to);
}

/**
 * 画面上の回転角をキーフォームの `angle` の変化に直すときの符号。親の変換や自分の反転で向きが
 * 裏返っている (ワールドの線形部分の行列式と、反転による行列式の符号が違う) と、角度を増やすと画面上では
 * 反時計回りに回る。
 */
export function rotationSense(frame: RotationFrame, reflectX: boolean, reflectY: boolean): 1 | -1 {
  const det = frame.x[0] * frame.y[1] - frame.x[1] * frame.y[0];
  const own = reflectX !== reflectY ? -1 : 1;
  return Math.sign(det) * own < 0 ? -1 : 1;
}

/** `center` からの距離の比 `|to - center| / |from - center|` (拡大率の取っ手)。`from` が中心なら null。 */
export function scaleFactor(center: Point, from: Point, to: Point): number | null {
  const a = Math.hypot(from[0] - center[0], from[1] - center[1]);
  if (a < 1e-9) return null;
  return Math.hypot(to[0] - center[0], to[1] - center[1]) / a;
}

/** 拡大率の下限 (コマンドは正の値だけを受け付ける)。 */
export const MIN_SCALE = 1e-3;

/** 角度を `step` 度に丸める (Shift を押しながらの回転)。 */
export function snapAngle(angle: number, step: number): number {
  return Math.round(angle / step) * step;
}
