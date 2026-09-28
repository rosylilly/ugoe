// WebPuppet のデフォーマの写像 (deformer_point / deformer_jacobian) を、physicsGeometry の関数の形にする。
// 今のパラメータ値 (と最後の update) での写像。
import type { WebPuppet } from "../../wasm";
import type { Point } from "../geometry";
import type { Jacobian, MapPoint } from "./physicsGeometry";

export function puppetMap(puppet: WebPuppet): MapPoint {
  return (deformer, p) => {
    if (deformer === null) return p;
    const q = puppet.deformer_point(deformer, p[0], p[1]);
    return q && q.length === 2 ? ([q[0] ?? 0, q[1] ?? 0] as Point) : null;
  };
}

export function puppetJacobian(puppet: WebPuppet): Jacobian {
  return (deformer, p) => {
    if (deformer === null) return [1, 0, 0, 1];
    const j = puppet.deformer_jacobian(deformer, p[0], p[1]);
    return j && j.length === 4 ? [j[0] ?? 0, j[1] ?? 0, j[2] ?? 0, j[3] ?? 0] : null;
  };
}
