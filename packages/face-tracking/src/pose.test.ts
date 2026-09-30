import { describe, expect, it } from "vitest";
import {
  bodyFromLandmarks,
  PoseConverter,
  type PoseLandmark,
  SHOULDER_NEUTRAL,
  SHOULDER_RANGE,
  VisibilityGate,
} from "./pose";

const W = 640;
const H = 480;
const ALL_VISIBLE = { shoulders: true, ears: true, hips: true };

/** 33 点の配列 (使わない点は原点)。 */
function points(set: Record<number, [number, number, number]>, visibility = 1): PoseLandmark[] {
  return Array.from({ length: 33 }, (_, i) => {
    const [x, y, z] = set[i] ?? [0, 0, 0];
    return { x, y, z, visibility };
  });
}

/**
 * 正面を向いて立つ人。画像ではピクセルで肩幅 200 (左肩が画像の右。ヨーの分だけ縮む)、耳の中点は肩の中点から
 * `drop` × 肩幅だけ上。world はメートルで肩幅 0.32、腰の中点が原点 (y は下向き)。
 */
function person({ drop = SHOULDER_NEUTRAL, rollDeg = 0, yawDeg = 0, leanZ = 0 } = {}) {
  const cx = 320;
  const cy = 240;
  const r = (rollDeg * Math.PI) / 180;
  const a = (yawDeg * Math.PI) / 180;
  // 横を向くと、画像の肩幅は cos(ヨー) 倍に縮む。
  const half = 100 * Math.cos(a);
  // 画像: 肩の中点のまわりに時計回り (y 下向き) に回す。
  const rot = (x: number, y: number): [number, number, number] => [
    (cx + x * Math.cos(r) - y * Math.sin(r)) / W,
    (cy + x * Math.sin(r) + y * Math.cos(r)) / H,
    0,
  ];
  const image = points({
    7: rot(40, -drop * 200),
    8: rot(-40, -drop * 200),
    11: rot(half, 0),
    12: rot(-half, 0),
    23: rot(60, 250),
    24: rot(-60, 250),
  });
  const world = points({
    11: [0.16 * Math.cos(a), -0.45, 0.16 * Math.sin(a) + leanZ],
    12: [-0.16 * Math.cos(a), -0.45, -0.16 * Math.sin(a) + leanZ],
    23: [0.1, 0, 0],
    24: [-0.1, 0, 0],
  });
  return { image, world };
}

describe("bodyFromLandmarks", () => {
  it("is neutral for a person facing the camera", () => {
    const { image, world } = person();
    const body = bodyFromLandmarks(image, world, W, H, ALL_VISIBLE);
    expect(body.yaw).toBeCloseTo(0);
    expect(body.pitch).toBeCloseTo(0);
    expect(body.roll).toBeCloseTo(0);
    expect(body.shoulderLeft).toBeCloseTo(0);
    expect(body.shoulderRight).toBeCloseTo(0);
  });

  it("takes the roll from the shoulder line in the image (clockwise is positive)", () => {
    const { image, world } = person({ rollDeg: 12 });
    const body = bodyFromLandmarks(image, world, W, H, ALL_VISIBLE);
    expect(body.roll).toBeCloseTo(12);
    // 体ごと傾けても、耳と肩の関係は変わらないので肩は中立のまま。
    expect(body.shoulderLeft).toBeCloseTo(0);
  });

  it("takes the yaw from the depth of the shoulders (own left is positive)", () => {
    // 自分の左を向くと、左肩がカメラから遠くなる (z が増える)。
    const { image, world } = person({ yawDeg: 25 });
    const body = bodyFromLandmarks(image, world, W, H, ALL_VISIBLE);
    expect(body.yaw).toBeCloseTo(25);
    // 画像の肩幅が縮んでも、ヨーで補正するので肩は中立のまま。
    expect(body.shoulderLeft).toBeCloseTo(0);
    const right = person({ yawDeg: -25 });
    expect(bodyFromLandmarks(right.image, right.world, W, H, ALL_VISIBLE).yaw).toBeCloseTo(-25);
  });

  it("takes the pitch from the hips to the shoulders (leaning back is positive)", () => {
    const { image, world } = person({ leanZ: 0.1 });
    const body = bodyFromLandmarks(image, world, W, H, ALL_VISIBLE);
    expect(body.pitch).toBeCloseTo((Math.atan2(0.1, 0.45) * 180) / Math.PI);
  });

  it("raises both shoulders when the ears come closer to the shoulder line", () => {
    const { image, world } = person({ drop: SHOULDER_NEUTRAL - SHOULDER_RANGE });
    const body = bodyFromLandmarks(image, world, W, H, ALL_VISIBLE);
    expect(body.shoulderLeft).toBeCloseTo(1);
    expect(body.shoulderRight).toBeCloseTo(1);
    const low = person({ drop: SHOULDER_NEUTRAL + SHOULDER_RANGE / 2 });
    expect(bodyFromLandmarks(low.image, low.world, W, H, ALL_VISIBLE).shoulderLeft).toBeCloseTo(
      -0.5,
    );
  });

  it("drops the values of parts that are not visible", () => {
    const { image, world } = person({ leanZ: 0.1 });
    const noHips = bodyFromLandmarks(image, world, W, H, { ...ALL_VISIBLE, hips: false });
    expect(noHips.pitch).toBeNull();
    expect(noHips.yaw).not.toBeNull();
    const noEars = bodyFromLandmarks(image, world, W, H, { ...ALL_VISIBLE, ears: false });
    expect(noEars.shoulderLeft).toBeNull();
    const noShoulders = bodyFromLandmarks(image, world, W, H, { ...ALL_VISIBLE, shoulders: false });
    expect(Object.values(noShoulders).every((v) => v === null)).toBe(true);
  });

  it("ignores a person seen from the back (left shoulder on the image's left)", () => {
    const { image, world } = person();
    const back = image.map((p) => ({ ...p, x: 1 - p.x }));
    const body = bodyFromLandmarks(back, world, W, H, ALL_VISIBLE);
    expect(body.roll).toBeNull();
    expect(body.yaw).toBeNull();
  });
});

describe("PoseConverter", () => {
  it("uses visibility with hysteresis and forgets it when the person is lost", () => {
    const converter = new PoseConverter();
    const { world } = person();
    const at = (visibility: number) => {
      const image = person().image.map((p) => ({ ...p, visibility }));
      return converter.convert({ landmarks: [image], worldLandmarks: [world] }, W, H);
    };
    expect(at(0.5)?.yaw).toBeNull();
    expect(at(0.7)?.yaw).not.toBeNull();
    // 一度見えたら 0.4 までは見えているとみなす。
    expect(at(0.5)?.yaw).not.toBeNull();
    expect(at(0.3)?.yaw).toBeNull();
    expect(at(0.7)?.yaw).not.toBeNull();
    expect(converter.convert({ landmarks: [], worldLandmarks: [] }, W, H)).toBeNull();
    expect(at(0.5)?.yaw).toBeNull();
  });

  it("gate starts hidden", () => {
    const gate = new VisibilityGate();
    expect(gate.update(0.59)).toBe(false);
    expect(gate.update(0.6)).toBe(true);
  });
});
