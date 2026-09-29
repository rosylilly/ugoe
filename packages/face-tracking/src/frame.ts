// トラッキングの 1 フレーム分の値と、MediaPipe Face Landmarker の結果からの変換。
//
// 値の約束は docs/spec/avatar-profile.md に合わせる:
// - ブレンドシェイプの `Left` / `Right` はトラッキングされた人自身の左右。
// - 頭の回転 (度) は「向きの約束」の符号: yaw は正でキャラクター (人) 自身の左を向く、pitch は正で上を向く、
//   roll は正で (画面で見て) 時計回りに傾く。
// パラメータへの変換は Rust (ugoe-wasm の `WebPuppet.set_tracking`) が行う。ここでは式を持たない。

/** 頭の回転 (度)。 */
export interface HeadPose {
  yaw: number;
  pitch: number;
  roll: number;
}

/** 1 フレーム分の入力。`blendShapes` は ARKit の名前 (MediaPipe のカテゴリ名) → 0〜1。 */
export interface TrackingFrame {
  head: HeadPose;
  blendShapes: Record<string, number>;
}

export const NEUTRAL_HEAD: HeadPose = { yaw: 0, pitch: 0, roll: 0 };

const RAD_TO_DEG = 180 / Math.PI;

function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

/**
 * MediaPipe の顔の変換行列 (`facialTransformationMatrixes` の `data`、4x4 の列優先) から頭の回転を求める。
 *
 * 行列は正準の顔の座標 (x: 画像の右、y: 上、z: カメラの方) からカメラの座標への変換で、正面を向いた顔では
 * 回転の部分が単位行列になる。回転を Y (ヨー)・X (ピッチ)・Z (ロール) の順の合成とみなして分解する。
 * 反転していない映像では、画像の右は写った人自身の左なので、顔が画像の右を向くと yaw が正になる。
 */
export function headPoseFromMatrix(data: ArrayLike<number>): HeadPose {
  // 列優先: 行 r・列 c の要素は data[c * 4 + r]。
  const m = (r: number, c: number) => data[c * 4 + r] ?? 0;
  // 顔の前方 (正準の z 軸) の向き。
  const fx = m(0, 2);
  const fy = m(1, 2);
  const fz = m(2, 2);
  const yaw = Math.atan2(fx, fz);
  const pitch = Math.asin(clamp(fy, -1, 1));
  // Z の回転は反時計回り (カメラから見て) が正なので、時計回りを正にするため符号を反転する。
  const roll = -Math.atan2(m(1, 0), m(1, 1));
  return { yaw: yaw * RAD_TO_DEG, pitch: pitch * RAD_TO_DEG, roll: roll * RAD_TO_DEG };
}

/** 名前の `Left` と `Right` を入れ替える (`eyeBlinkLeft` ↔ `eyeBlinkRight`、`mouthLeft` ↔ `mouthRight`)。 */
export function swapSide(name: string): string {
  if (name.endsWith("Left")) return `${name.slice(0, -4)}Right`;
  if (name.endsWith("Right")) return `${name.slice(0, -5)}Left`;
  return name;
}

/**
 * 左右反転した映像から得た値を、反転していない映像の値に直す (左右の名前を入れ替え、yaw と roll の符号を反転する)。
 * 2 回かけると元に戻る。
 */
export function unmirrorFrame(frame: TrackingFrame): TrackingFrame {
  const blendShapes: Record<string, number> = {};
  for (const [name, value] of Object.entries(frame.blendShapes))
    blendShapes[swapSide(name)] = value;
  const { yaw, pitch, roll } = frame.head;
  return { head: { yaw: -yaw, pitch, roll: -roll }, blendShapes };
}

/** MediaPipe の `FaceLandmarkerResult` のうち、ここで使う部分。 */
export interface FaceLandmarkerOutput {
  faceBlendshapes?: { categories: { categoryName: string; score: number }[] }[];
  facialTransformationMatrixes?: { data: ArrayLike<number> }[];
}

/** Face Landmarker の結果の 1 人目の顔をフレームにする。顔が無ければ `null`。`_neutral` は除く。 */
export function frameFromLandmarker(result: FaceLandmarkerOutput): TrackingFrame | null {
  const shapes = result.faceBlendshapes?.[0];
  const matrix = result.facialTransformationMatrixes?.[0];
  if (shapes === undefined || matrix === undefined) return null;
  const blendShapes: Record<string, number> = {};
  for (const { categoryName, score } of shapes.categories) {
    if (categoryName !== "_neutral") blendShapes[categoryName] = score;
  }
  return { head: headPoseFromMatrix(matrix.data), blendShapes };
}
