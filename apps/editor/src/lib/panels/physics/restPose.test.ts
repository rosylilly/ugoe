import { describe, expect, it } from "vitest";
import type { PhysicsSettings, RestPoseCollision } from "../../generated/types";
import { formatSegments, restPoseKey, restPoseTitle } from "./restPose";

// 表記が参照するフィールドだけを持つ設定 (型検査のためにキャストする)。
const settings = {
  rigs: [{ id: "Hair", name: "前髪" }],
  colliders: [{ id: "Shoulder" }],
} as unknown as PhysicsSettings;

const found: RestPoseCollision = { rig: "Hair", collider: "Shoulder", particles: [1, 2] };

describe("restPoseTitle", () => {
  it("labels the rig and the collider with names and IDs", () => {
    expect(restPoseTitle(found, settings)).toBe("前髪 (Hair) hits Shoulder");
  });

  it("falls back to the IDs without settings", () => {
    expect(restPoseTitle(found, null)).toBe("Hair hits Shoulder");
  });
});

describe("formatSegments", () => {
  it("counts segments by their child particle", () => {
    expect(formatSegments([2])).toBe("segment 2");
    expect(formatSegments([1, 2])).toBe("segments 1, 2");
  });
});

describe("restPoseKey", () => {
  it("distinguishes rig and collider pairs", () => {
    expect(restPoseKey(found)).not.toBe(restPoseKey({ ...found, collider: "Chest" }));
  });
});
