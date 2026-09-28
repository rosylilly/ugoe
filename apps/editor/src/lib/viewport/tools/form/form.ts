// フォーム編集の点の選択 (docs/editor.md#キーフォームの編集)。DOM や wasm に依存しない (form.test.ts)。
import { type Point, updateVertexSelection } from "../geometry";

/** 点を押したときの選択の更新と、そのままドラッグで動かすか。 */
export function pressPoint(
  picked: readonly number[],
  hit: number,
  toggle: boolean,
): { picked: number[]; drag: boolean } {
  if (toggle) {
    const next = updateVertexSelection(picked, [hit], true);
    // Shift で選択から外した点はつかまない。
    return { picked: next, drag: next.includes(hit) };
  }
  // 選択中の点を押したら選択のまま (まとめて動かす)、そうでなければその点だけを選ぶ。
  return { picked: picked.includes(hit) ? [...picked] : [hit], drag: true };
}

/** 選択の番号のうち、点の数 `count` の中にあるもの (形が変わった後の古い番号を捨てる)。 */
export function validIndices(picked: readonly number[], count: number): number[] {
  return picked.filter((i) => Number.isInteger(i) && i >= 0 && i < count);
}

/** 三角形 (`indices` の 3 つ組) を塗る SVG のパス。点が足りない三角形は飛ばす。 */
export function trianglesPath(points: readonly Point[], indices: ArrayLike<number>): string {
  let d = "";
  for (let t = 0; t + 2 < indices.length; t += 3) {
    const tri = [indices[t], indices[t + 1], indices[t + 2]].map((i) => points[i ?? -1]);
    if (tri.some((p) => !p)) continue;
    const [a, b, c] = tri as [Point, Point, Point];
    d += `M${a[0]},${a[1]}L${b[0]},${b[1]}L${c[0]},${c[1]}Z`;
  }
  return d;
}
