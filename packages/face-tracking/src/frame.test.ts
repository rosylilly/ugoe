import { describe, expect, it } from "vitest";
import {
  frameFromLandmarker,
  headPoseFromMatrix,
  setPuppetTracking,
  swapSide,
  unmirrorFrame,
} from "./frame";

const DEG = Math.PI / 180;

/** 回転行列 (3x3、行優先) を MediaPipe の 4x4 列優先の配列にする。 */
function columnMajor(r: number[][]): number[] {
  const out = new Array<number>(16).fill(0);
  for (let row = 0; row < 3; row++) {
    for (let col = 0; col < 3; col++) out[col * 4 + row] = r[row]?.[col] ?? 0;
  }
  out[15] = 1;
  return out;
}

function mul(a: number[][], b: number[][]): number[][] {
  return a.map((row) => [0, 1, 2].map((j) => row.reduce((s, v, k) => s + v * (b[k]?.[j] ?? 0), 0)));
}

const rotY = (a: number) => [
  [Math.cos(a), 0, Math.sin(a)],
  [0, 1, 0],
  [-Math.sin(a), 0, Math.cos(a)],
];
const rotX = (a: number) => [
  [1, 0, 0],
  [0, Math.cos(a), -Math.sin(a)],
  [0, Math.sin(a), Math.cos(a)],
];
const rotZ = (a: number) => [
  [Math.cos(a), -Math.sin(a), 0],
  [Math.sin(a), Math.cos(a), 0],
  [0, 0, 1],
];

describe("headPoseFromMatrix", () => {
  it("is zero for the identity", () => {
    const pose = headPoseFromMatrix(columnMajor(rotY(0)));
    expect(pose.yaw).toBeCloseTo(0);
    expect(pose.pitch).toBeCloseTo(0);
    expect(pose.roll).toBeCloseTo(0);
  });

  it("turning the face to the image right is a positive yaw", () => {
    // 前方 (z) が +x の方へ回る
    expect(headPoseFromMatrix(columnMajor(rotY(20 * DEG))).yaw).toBeCloseTo(20);
  });

  it("looking up is a positive pitch", () => {
    // 前方 (z) が +y (上) の方へ回る
    expect(headPoseFromMatrix(columnMajor(rotX(-15 * DEG))).pitch).toBeCloseTo(15);
  });

  it("a clockwise tilt on screen is a positive roll", () => {
    // カメラから見て時計回り = z 軸まわりの負の回転 (頭の上 +y が +x の方へ倒れる)
    expect(headPoseFromMatrix(columnMajor(rotZ(-10 * DEG))).roll).toBeCloseTo(10);
  });

  it("separates combined rotations (Y, then X, then Z)", () => {
    const r = mul(mul(rotY(25 * DEG), rotX(-10 * DEG)), rotZ(-5 * DEG));
    const pose = headPoseFromMatrix(columnMajor(r));
    expect(pose.yaw).toBeCloseTo(25);
    expect(pose.pitch).toBeCloseTo(10);
    expect(pose.roll).toBeCloseTo(5);
  });
});

describe("mirroring", () => {
  it("swaps Left and Right suffixes only", () => {
    expect(swapSide("eyeBlinkLeft")).toBe("eyeBlinkRight");
    expect(swapSide("mouthRight")).toBe("mouthLeft");
    expect(swapSide("jawOpen")).toBe("jawOpen");
  });

  it("negates yaw and roll and is its own inverse", () => {
    const frame = {
      head: { yaw: 10, pitch: 5, roll: -3 },
      blendShapes: { eyeBlinkLeft: 0.8, eyeBlinkRight: 0.1, jawOpen: 0.4 },
    };
    const m = unmirrorFrame(frame);
    expect(m.head).toEqual({ yaw: -10, pitch: 5, roll: 3 });
    expect(m.blendShapes).toEqual({ eyeBlinkLeft: 0.1, eyeBlinkRight: 0.8, jawOpen: 0.4 });
    expect(unmirrorFrame(m)).toEqual(frame);
  });
});

describe("frameFromLandmarker", () => {
  it("takes the first face and drops _neutral", () => {
    const frame = frameFromLandmarker({
      faceBlendshapes: [
        {
          categories: [
            { categoryName: "_neutral", score: 0.9 },
            { categoryName: "jawOpen", score: 0.3 },
          ],
        },
      ],
      facialTransformationMatrixes: [{ data: columnMajor(rotY(0)) }],
    });
    expect(frame?.blendShapes).toEqual({ jawOpen: 0.3 });
    expect(frame?.head.yaw).toBeCloseTo(0);
  });

  it("is null without a face", () => {
    expect(
      frameFromLandmarker({ faceBlendshapes: [], facialTransformationMatrixes: [] }),
    ).toBeNull();
  });
});

describe("body", () => {
  const body = { yaw: 10, pitch: -3, roll: 4, shoulderLeft: 0.8, shoulderRight: null };
  const frame = { head: { yaw: 1, pitch: 2, roll: 3 }, blendShapes: { jawOpen: 0.5 }, body };

  it("mirroring swaps the shoulders and negates yaw and roll", () => {
    const out = unmirrorFrame(frame);
    expect(out.body).toEqual({
      yaw: -10,
      pitch: -3,
      roll: -4,
      shoulderLeft: null,
      shoulderRight: 0.8,
    });
    expect(unmirrorFrame(out).body).toEqual(body);
    expect(unmirrorFrame({ head: frame.head, blendShapes: {} }).body).toBeUndefined();
  });

  it("passes the body to set_tracking as optional trailing arguments", () => {
    const calls: unknown[][] = [];
    const puppet = { set_tracking: (...args: unknown[]) => calls.push(args) };
    setPuppetTracking(puppet, frame);
    setPuppetTracking(puppet, { head: frame.head, blendShapes: {} });
    expect(calls[0]?.slice(2)).toEqual([1, 2, 3, 10, -3, 4, 0.8, null]);
    expect(calls[1]?.slice(2)).toEqual([1, 2, 3, null, null, null, null, null]);
    expect(Array.from(calls[0]?.[1] as Float32Array)).toEqual([0.5]);
  });
});
