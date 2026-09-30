import { describe, expect, it } from "vitest";
import {
  headRotationFromMatrix,
  mirrorTracking,
  trackingArguments,
  trackingFromFaceLandmarker,
} from "./tracking.js";

const DEG = Math.PI / 180;
type M3 = number[][];

const mul = (a: M3, b: M3): M3 =>
  a.map((row) => [0, 1, 2].map((c) => row.reduce((s, v, k) => s + v * (b[k]?.[c] ?? 0), 0)));
const rx = (t: number): M3 => [
  [1, 0, 0],
  [0, Math.cos(t), -Math.sin(t)],
  [0, Math.sin(t), Math.cos(t)],
];
const ry = (t: number): M3 => [
  [Math.cos(t), 0, Math.sin(t)],
  [0, 1, 0],
  [-Math.sin(t), 0, Math.cos(t)],
];
const rz = (t: number): M3 => [
  [Math.cos(t), -Math.sin(t), 0],
  [Math.sin(t), Math.cos(t), 0],
  [0, 0, 1],
];

/** 3x3 の回転 (と拡大・平行移動) を 4x4 の列優先の配列にする。 */
function columnMajor(r: M3, scale = 1, t: [number, number, number] = [0, 0, 0]): number[] {
  const out: number[] = [];
  for (let c = 0; c < 4; c++) {
    for (let row = 0; row < 4; row++) {
      if (c === 3) out.push(row < 3 ? (t[row] ?? 0) : 1);
      else out.push(row < 3 ? (r[row]?.[c] ?? 0) * scale : 0);
    }
  }
  return out;
}

describe("headRotationFromMatrix", () => {
  it("is zero for the identity", () => {
    const r = headRotationFromMatrix(columnMajor(rx(0)));
    expect(r.yaw).toBeCloseTo(0);
    expect(r.pitch).toBeCloseTo(0);
    expect(r.roll).toBeCloseTo(0);
  });

  it("follows the avatar profile signs for each axis", () => {
    // 顔の正面 (+z) が画像の右 (+x) へ回る = 写った人自身の左を向く → 正のヨー
    expect(headRotationFromMatrix(columnMajor(ry(20 * DEG))).yaw).toBeCloseTo(20);
    // 正面が上 (+y) へ回る → 正のピッチ (+x まわりの負の回転)
    expect(headRotationFromMatrix(columnMajor(rx(-15 * DEG))).pitch).toBeCloseTo(15);
    // y 上向きの画像で時計回り (+z まわりの負の回転) → 正のロール
    expect(headRotationFromMatrix(columnMajor(rz(-10 * DEG))).roll).toBeCloseTo(10);
  });

  it("decomposes combined rotations and ignores scale and translation", () => {
    const r = mul(mul(ry(25 * DEG), rx(-12 * DEG)), rz(8 * DEG));
    const pose = headRotationFromMatrix(columnMajor(r, 3.5, [1, -2, -40]));
    expect(pose.yaw).toBeCloseTo(25);
    expect(pose.pitch).toBeCloseTo(12);
    expect(pose.roll).toBeCloseTo(-8);
  });
});

describe("trackingFromFaceLandmarker", () => {
  const result = {
    faceBlendshapes: [
      {
        categories: [
          { categoryName: "_neutral", score: 0.1 },
          { categoryName: "eyeBlinkLeft", score: 0.8 },
          { categoryName: "eyeBlinkRight", score: 0.2 },
          { categoryName: "jawOpen", score: 0.5 },
        ],
      },
    ],
    facialTransformationMatrixes: [{ data: columnMajor(mul(ry(20 * DEG), rz(-5 * DEG))) }],
  };

  it("takes the first face's blendshapes (without _neutral) and head rotation", () => {
    const input = trackingFromFaceLandmarker(result);
    expect(input?.blendshapes).toEqual({ eyeBlinkLeft: 0.8, eyeBlinkRight: 0.2, jawOpen: 0.5 });
    expect(input?.head.yaw).toBeCloseTo(20);
    expect(input?.head.roll).toBeCloseTo(5);
  });

  it("swaps sides and flips yaw and roll for a mirrored input", () => {
    const input = trackingFromFaceLandmarker(result, { mirrored: true });
    expect(input?.blendshapes).toEqual({ eyeBlinkRight: 0.8, eyeBlinkLeft: 0.2, jawOpen: 0.5 });
    expect(input?.head.yaw).toBeCloseTo(-20);
    expect(input?.head.pitch).toBeCloseTo(0);
    expect(input?.head.roll).toBeCloseTo(-5);
  });

  it("returns null without a face or without the matrix", () => {
    expect(
      trackingFromFaceLandmarker({ faceBlendshapes: [], facialTransformationMatrixes: [] }),
    ).toBe(null);
    expect(trackingFromFaceLandmarker({ faceBlendshapes: result.faceBlendshapes })).toBe(null);
  });
});

describe("mirrorTracking", () => {
  it("swaps the shoulders and flips the body's yaw and roll", () => {
    const m = mirrorTracking({
      blendshapes: { mouthSmileLeft: 1 },
      head: { yaw: 10, pitch: 5, roll: 3 },
      body: { yaw: 4, roll: -2, shoulderLeft: 0.7 },
    });
    expect(m.blendshapes).toEqual({ mouthSmileRight: 1 });
    expect(m.head).toEqual({ yaw: -10, pitch: 5, roll: -3 });
    expect(m.body).toEqual({
      yaw: -4,
      pitch: undefined,
      roll: 2,
      shoulderLeft: undefined,
      shoulderRight: 0.7,
    });
  });
});

describe("trackingArguments", () => {
  it("builds the WebPuppet.set_tracking arguments", () => {
    const [names, scores, ...rest] = trackingArguments({
      blendshapes: { jawOpen: 0.25, eyeBlinkLeft: 1 },
      head: { yaw: 1, pitch: 2, roll: 3 },
      body: { roll: 4, shoulderRight: 0.5 },
    });
    expect(names).toEqual(["jawOpen", "eyeBlinkLeft"]);
    expect(Array.from(scores)).toEqual([0.25, 1]);
    expect(rest).toEqual([1, 2, 3, undefined, undefined, 4, undefined, 0.5]);
  });
});
