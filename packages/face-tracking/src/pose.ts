// 体のトラッキング (T0645): MediaPipe Pose Landmarker の結果から体の値 (BodyPose) を求める。
//
// 式と、実際の画像で確かめたことは docs/spec/avatar-profile.md#体のトラッキング-mediapipe-pose-landmarker に書く。
// 使う点は 7・8 (左右の耳)、11・12 (左右の肩)、23・24 (左右の腰)。MediaPipe の left / right は写った人自身の左右
// (反転していない映像では、左肩は画像の右に写る)。
//
// - ロール (BodyAngleZ): 画像 (ピクセル) での肩の線の傾き。画面で時計回りが正。
// - ヨー (BodyAngleX): worldLandmarks (メートル、腰の中点が原点、z はカメラから遠いほど大きい) での肩の線の
//   奥行きの傾き。写った人が自分の左を向くと (左肩が奥へ行き) 正。
// - ピッチ (BodyAngleY): worldLandmarks での、腰の中点から肩の中点への向きの前後の傾き。後ろへ反ると正。
//   腰が映っていなければ無い。
// - 肩の上下: 画像での、耳の中点から肩の中点までの肩の線に垂直な距離を、肩幅 (体のヨーで補正) で割った値の
//   基準からの減り。左右の肩を別々には求めない (片方の肩だけを上げた動きは、肩の線の傾き = ロールと区別できない
//   ため、ロールに任せる)。左右に同じ値を入れる。
//
// ピッチと肩の上下は、カメラの高さや体格で基準が大きく変わるので、キャリブレーションした後だけ使う
// (calibration.ts の applyCalibration)。
import { BODY_KEYS, type BodyPose, NO_BODY } from "./frame";

/** Pose Landmarker の 1 点 (正規化した画像座標、または worldLandmarks のメートル)。 */
export interface PoseLandmark {
  x: number;
  y: number;
  z: number;
  visibility?: number;
}

/** MediaPipe の `PoseLandmarkerResult` のうち、ここで使う部分。 */
export interface PoseLandmarkerOutput {
  landmarks?: PoseLandmark[][];
  worldLandmarks?: PoseLandmark[][];
}

const LEFT_EAR = 7;
const RIGHT_EAR = 8;
const LEFT_SHOULDER = 11;
const RIGHT_SHOULDER = 12;
const LEFT_HIP = 23;
const RIGHT_HIP = 24;

const RAD_TO_DEG = 180 / Math.PI;

/** 見えているとみなし始める visibility と、見えなくなったとみなす visibility (ちらつきを抑える)。 */
export const VISIBILITY_ON = 0.6;
export const VISIBILITY_OFF = 0.4;
/**
 * 肩の上下の目安: 耳から肩までの高さ ÷ 肩幅の、中立での値と、すくめたときの減り。
 * 中立の値は MediaPipe の試験用の画像などで 0.60〜0.65 (pose_landmarker_lite)。減りは肩幅の約 2 割
 * (すくめると肩が 6〜7 cm 上がる) とした。キャリブレーションで中立を差し引くので、中立の値は記録の目安にだけ効く。
 */
export const SHOULDER_NEUTRAL = 0.62;
export const SHOULDER_RANGE = 0.2;
/** 肩幅をヨーで補正するときの cos の下限 (横を向いたときに割りすぎないように)。 */
const MIN_COS_YAW = 0.5;

/** 点の組ごとの「見えている」の状態 (visibility のヒステリシス)。 */
export class VisibilityGate {
  #visible = false;

  update(visibility: number): boolean {
    this.#visible = this.#visible ? visibility >= VISIBILITY_OFF : visibility >= VISIBILITY_ON;
    return this.#visible;
  }

  reset(): void {
    this.#visible = false;
  }
}

function visibility(points: readonly PoseLandmark[], indices: readonly number[]): number {
  return Math.min(...indices.map((i) => points[i]?.visibility ?? 0));
}

/**
 * Pose Landmarker の結果から体の値を求める。`width` / `height` は入力の画像の大きさ (ピクセル。
 * 正規化した座標の縦横比を戻すため)。点の組ごとの見えている状態をフレームをまたいで保つ。
 */
export class PoseConverter {
  readonly #shoulders = new VisibilityGate();
  readonly #ears = new VisibilityGate();
  readonly #hips = new VisibilityGate();

  /** 人がいなければ `null`。 */
  convert(result: PoseLandmarkerOutput, width: number, height: number): BodyPose | null {
    const image = result.landmarks?.[0];
    const world = result.worldLandmarks?.[0];
    if (image === undefined || world === undefined) {
      this.reset();
      return null;
    }
    return bodyFromLandmarks(image, world, width, height, {
      shoulders: this.#shoulders.update(visibility(image, [LEFT_SHOULDER, RIGHT_SHOULDER])),
      ears: this.#ears.update(visibility(image, [LEFT_EAR, RIGHT_EAR])),
      hips: this.#hips.update(visibility(image, [LEFT_HIP, RIGHT_HIP])),
    });
  }

  reset(): void {
    this.#shoulders.reset();
    this.#ears.reset();
    this.#hips.reset();
  }
}

/** どの点の組が見えているか。 */
export interface VisibleParts {
  shoulders: boolean;
  ears: boolean;
  hips: boolean;
}

/**
 * 1 人分の点 (`image` は正規化した画像座標、`world` は worldLandmarks) から体の値を求める。
 * 見えていない組から求める値は `null`。
 */
export function bodyFromLandmarks(
  image: readonly PoseLandmark[],
  world: readonly PoseLandmark[],
  width: number,
  height: number,
  visible: VisibleParts,
): BodyPose {
  const body: BodyPose = { ...NO_BODY };
  const px = (i: number) => {
    const p = image[i];
    return p === undefined ? null : { x: p.x * width, y: p.y * height };
  };
  const ls = px(LEFT_SHOULDER);
  const rs = px(RIGHT_SHOULDER);
  const wl = world[LEFT_SHOULDER];
  const wr = world[RIGHT_SHOULDER];
  if (!visible.shoulders || ls === null || rs === null || wl === undefined || wr === undefined) {
    return body;
  }
  // 左肩が画像の右に無い (背中を向けている・左右を取り違えた) ときは使わない。
  const dx = ls.x - rs.x;
  const dy = ls.y - rs.y;
  if (!(dx > 0)) return body;
  body.roll = Math.atan2(dy, dx) * RAD_TO_DEG;
  const yaw = Math.atan2(wl.z - wr.z, wl.x - wr.x);
  body.yaw = yaw * RAD_TO_DEG;

  const wlh = world[LEFT_HIP];
  const wrh = world[RIGHT_HIP];
  if (visible.hips && wlh !== undefined && wrh !== undefined) {
    // 腰の中点から肩の中点へ (y は下向き)。
    const ty = (wl.y + wr.y - wlh.y - wrh.y) / 2;
    const tz = (wl.z + wr.z - wlh.z - wrh.z) / 2;
    body.pitch = Math.atan2(tz, -ty) * RAD_TO_DEG;
  }

  const le = px(LEFT_EAR);
  const re = px(RIGHT_EAR);
  if (visible.ears && le !== null && re !== null) {
    const shoulderWidth = Math.hypot(dx, dy);
    // 耳の中点から肩の中点へのベクトルの、肩の線に垂直な成分 (下向きが正)。
    const mx = (ls.x + rs.x - le.x - re.x) / 2;
    const my = (ls.y + rs.y - le.y - re.y) / 2;
    const drop = (my * dx - mx * dy) / shoulderWidth;
    const widthAtFront = shoulderWidth / Math.max(MIN_COS_YAW, Math.cos(yaw));
    const raise = (SHOULDER_NEUTRAL - drop / widthAtFront) / SHOULDER_RANGE;
    body.shoulderLeft = raise;
    body.shoulderRight = raise;
  }
  for (const key of BODY_KEYS) {
    const v = body[key];
    if (v !== null && !Number.isFinite(v)) body[key] = null;
  }
  return body;
}
