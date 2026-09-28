// ビューポートのツールが使う、DOM や wasm に依存しない幾何の補助 (Vitest でテストする)。
// 頂点位置は WebPuppet.drawable_positions と同じ平坦な配列 [x0, y0, x1, y1, ...]。

export type Point = [number, number];

/** 2 点を対角とする矩形 (向きは問わない)。 */
export interface Box {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

/** 平坦な配列の頂点数。 */
export function vertexCount(positions: ArrayLike<number>): number {
  return Math.floor(positions.length / 2);
}

export function vertexAt(positions: ArrayLike<number>, i: number): Point {
  return [positions[2 * i] ?? Number.NaN, positions[2 * i + 1] ?? Number.NaN];
}

/** `(x, y)` から `radius` 以内で最も近い頂点の番号 (同じ距離なら小さい番号)。 */
export function nearestVertex(
  positions: ArrayLike<number>,
  x: number,
  y: number,
  radius: number,
): number | null {
  let best: number | null = null;
  let bestDistance = radius * radius;
  for (let i = 0; i < vertexCount(positions); i++) {
    const [vx, vy] = vertexAt(positions, i);
    const d = (vx - x) ** 2 + (vy - y) ** 2;
    if (best === null ? d <= bestDistance : d < bestDistance) {
      best = i;
      bestDistance = d;
    }
  }
  return best;
}

/** 矩形 (辺の上を含む) に入る頂点の番号 (昇順)。 */
export function verticesInBox(positions: ArrayLike<number>, box: Box): number[] {
  const [minX, maxX] = [Math.min(box.x0, box.x1), Math.max(box.x0, box.x1)];
  const [minY, maxY] = [Math.min(box.y0, box.y1), Math.max(box.y0, box.y1)];
  const out: number[] = [];
  for (let i = 0; i < vertexCount(positions); i++) {
    const [x, y] = vertexAt(positions, i);
    if (x >= minX && x <= maxX && y >= minY && y <= maxY) out.push(i);
  }
  return out;
}

/** 三角形の辺 (重複なし、`[小さい番号, 大きい番号]`、出てきた順)。 */
export function meshEdges(indices: ArrayLike<number>): [number, number][] {
  const seen = new Set<string>();
  const out: [number, number][] = [];
  for (let t = 0; t + 2 < indices.length; t += 3) {
    for (const k of [0, 1, 2]) {
      const a = indices[t + k] ?? 0;
      const b = indices[t + ((k + 1) % 3)] ?? 0;
      const e: [number, number] = a < b ? [a, b] : [b, a];
      const key = `${e[0]},${e[1]}`;
      if (a !== b && !seen.has(key)) {
        seen.add(key);
        out.push(e);
      }
    }
  }
  return out;
}

/** 輪郭の辺 (1 つの三角形だけが使う辺)。選択枠の表示に使う。 */
export function boundaryEdges(indices: ArrayLike<number>): [number, number][] {
  const count = new Map<string, number>();
  const edges: [number, number][] = [];
  for (let t = 0; t + 2 < indices.length; t += 3) {
    for (const k of [0, 1, 2]) {
      const a = indices[t + k] ?? 0;
      const b = indices[t + ((k + 1) % 3)] ?? 0;
      if (a === b) continue;
      const key = a < b ? `${a},${b}` : `${b},${a}`;
      const n = count.get(key) ?? 0;
      count.set(key, n + 1);
      if (n === 0) edges.push([a, b]);
    }
  }
  return edges.filter(([a, b]) => count.get(a < b ? `${a},${b}` : `${b},${a}`) === 1);
}

/**
 * Alt + クリックで重なった要素を順に選ぶ: `hits` (手前から奥へ) のうち、`current` の次のもの。
 * `current` が `hits` に無ければ先頭 (いちばん手前)。
 */
export function cycleHit<T>(hits: readonly T[], current: T | null): T | null {
  if (hits.length === 0) return null;
  const i = current === null ? -1 : hits.indexOf(current);
  return hits[(i + 1) % hits.length] ?? null;
}

/** 選択中の番号の集合を、クリックの修飾キーに合わせて更新する (Shift は追加・解除の切り替え)。 */
export function updateVertexSelection(
  current: readonly number[],
  picked: readonly number[],
  toggle: boolean,
): number[] {
  if (!toggle) return [...picked].sort((a, b) => a - b);
  const set = new Set(current);
  // 矩形選択 (複数) の Shift は追加だけ、1 点のクリックは切り替え
  if (picked.length === 1 && set.has(picked[0] as number)) set.delete(picked[0] as number);
  else for (const i of picked) set.add(i);
  return [...set].sort((a, b) => a - b);
}
