// 静止姿勢で衝突体に当たっている鎖の警告の表記 (docs/editor.md#物理の警告)。
// DOM に依存しない純粋な関数だけを置く (restPose.test.ts で検査する)。
import type { PhysicsSettings, RestPoseCollision } from "../../generated/types";
import { labelOf } from "../../labels";

/** 警告の見出し (`Hair hits Shoulder`)。リグと衝突体は表示名と ID で書く。 */
export function restPoseTitle(c: RestPoseCollision, settings: PhysicsSettings | null): string {
  return `${labelOf(settings?.rigs, c.rig)} hits ${labelOf(settings?.colliders, c.collider)}`;
}

/** 当たっている区間の表記。区間は子の粒子の添字で数える (`segment 2`・`segments 1, 2`)。 */
export function formatSegments(particles: readonly number[]): string {
  return `${particles.length === 1 ? "segment" : "segments"} ${particles.join(", ")}`;
}

/** 行の識別子 (リグと衝突体の組)。 */
export function restPoseKey(c: RestPoseCollision): string {
  return `${c.rig}\u0000${c.collider}`;
}
