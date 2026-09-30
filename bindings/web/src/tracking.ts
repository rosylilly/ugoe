// フェイストラッキングの入力 (`<ugoe-avatar>` の `setTracking`・`WebPuppet.set_tracking`) と、
// MediaPipe Face Landmarker の結果からの変換。DOM と wasm に触れないので、Node でテストする (tracking.test.ts)。
//
// このパッケージはトラッカーを持たない (ADR 0025 の「フェイストラッキング」)。カメラと MediaPipe などのトラッカーは
// ホストが用意し、結果をここの型に変えて渡す。MediaPipe の型は import せず、使う形だけを `FaceLandmarkerResultLike` に書く。
// 対応付け (ブレンドシェイプ → パラメータ) は ugoe-wasm (Rust) が行う。向きと左右の約束は
// docs/spec/avatar-profile.md#向きと左右の約束、ホストが行うこと (平滑化・キャリブレーション) は同じ文書の #実装の範囲。

/** 頭や体の回転 (度)。符号はアバタープロファイルの約束 (`AngleX` / `AngleY` / `AngleZ` と同じ向き)。 */
export interface TrackingRotation {
  /** 正でトラッキングされた人自身の左を向く (反転していない映像では画像の右)。 */
  yaw: number;
  /** 正で上を向く。 */
  pitch: number;
  /** 正で時計回りに傾く (反転していない映像で見て)。 */
  roll: number;
}

/** 体のトラッキングの入力。どれも省略できる。 */
export interface TrackingBody {
  /** 体の回転 (度)。省略した軸は頭の回転 × 0.3。 */
  yaw?: number;
  pitch?: number;
  roll?: number;
  /** 肩の上下 (0 が中立、1 ですくめた状態)。省略すると書き込まない。 */
  shoulderLeft?: number;
  shoulderRight?: number;
}

/** トラッキングの 1 フレーム分の入力。 */
export interface TrackingInput {
  /**
   * ARKit のブレンドシェイプの名前と値 (0〜1)。MediaPipe のカテゴリ名をそのまま使える。
   * 知らない名前は無視し、無い名前は 0。`Left` / `Right` はトラッキングされた人自身の左右。
   */
  blendshapes: Record<string, number>;
  head: TrackingRotation;
  body?: TrackingBody;
}

/** MediaPipe の `FaceLandmarkerResult` のうち、ここで使う部分 (`@mediapipe/tasks-vision` の型を import しないため)。 */
export interface FaceLandmarkerResultLike {
  faceBlendshapes?: readonly {
    categories: readonly { categoryName: string; score: number }[];
  }[];
  /** 顔の変換行列 (4x4、列優先)。`outputFacialTransformationMatrixes: true` で作ったときだけある。 */
  facialTransformationMatrixes?: readonly { data: ArrayLike<number> }[];
}

/** `trackingFromFaceLandmarker` の設定。 */
export interface FaceLandmarkerOptions {
  /**
   * トラッカーに左右反転した映像を入れたなら `true` (既定は `false`)。左右の名前を入れ替え、ヨーとロールの符号を反転して、
   * トラッキングされた人自身の左右に直す。表示だけを反転しているなら `false` のまま。
   */
  mirrored?: boolean;
}

const RAD_TO_DEG = 180 / Math.PI;

/**
 * 顔の変換行列 (4x4、列優先。顔の正準モデルの座標 → カメラ座標) から頭の回転を求める。
 *
 * カメラ座標は x が画像の右、y が上、z がカメラの手前 (MediaPipe の顔の幾何の約束)。
 * 回転を `Ry(ヨー) · Rx(−ピッチ) · Rz(−ロール)` の順の積として分解する。行列が一様な拡大を含んでもよい。
 * 顔の正面 (正準モデルの +z) が画像の右を向くと正のヨー、上を向くと正のピッチ、画像の上で時計回りに傾くと正のロール。
 */
export function headRotationFromMatrix(data: ArrayLike<number>): TrackingRotation {
  // 列優先: 行 r・列 c の要素は data[c * 4 + r]
  const at = (r: number, c: number) => data[c * 4 + r] ?? 0;
  // 正面の向き = 3 列目 (正規化して拡大を除く)
  const fx = at(0, 2);
  const fy = at(1, 2);
  const fz = at(2, 2);
  const len = Math.hypot(fx, fy, fz) || 1;
  const yaw = Math.atan2(fx, fz);
  const pitch = Math.asin(Math.min(1, Math.max(-1, fy / len)));
  // Ry は 2 行目を変えないので、2 行目の (0 列, 1 列) = cos(ピッチ) · (sin ρ, cos ρ)。ρ = −ロール
  const roll = -Math.atan2(at(1, 0), at(1, 1));
  return { yaw: yaw * RAD_TO_DEG, pitch: pitch * RAD_TO_DEG, roll: roll * RAD_TO_DEG };
}

/**
 * MediaPipe Face Landmarker の結果の最初の顔を、トラッキングの入力にする。顔が無ければ `null`。
 * 結果はブレンドシェイプと変換行列の両方を含む必要がある
 * (`outputFaceBlendshapes: true`・`outputFacialTransformationMatrixes: true` で作る)。
 */
export function trackingFromFaceLandmarker(
  result: FaceLandmarkerResultLike,
  options: FaceLandmarkerOptions = {},
): TrackingInput | null {
  const shapes = result.faceBlendshapes?.[0];
  const matrix = result.facialTransformationMatrixes?.[0];
  if (!shapes || !matrix) return null;
  const blendshapes: Record<string, number> = {};
  for (const { categoryName, score } of shapes.categories) {
    if (categoryName !== "_neutral") blendshapes[categoryName] = score;
  }
  const input: TrackingInput = { blendshapes, head: headRotationFromMatrix(matrix.data) };
  return options.mirrored ? mirrorTracking(input) : input;
}

/** 名前の `Left` と `Right` を入れ替える (`eyeBlinkLeft` ↔ `eyeBlinkRight`)。 */
function swapSide(name: string): string {
  return name.replace(/Left|Right/g, (s) => (s === "Left" ? "Right" : "Left"));
}

/**
 * 左右反転した映像から作った入力を、トラッキングされた人自身の左右に直す (逆も同じ)。
 * ブレンドシェイプの `Left` / `Right` と左右の肩を入れ替え、頭と体のヨーとロールの符号を反転する。
 */
export function mirrorTracking(input: TrackingInput): TrackingInput {
  const blendshapes: Record<string, number> = {};
  for (const [name, v] of Object.entries(input.blendshapes)) blendshapes[swapSide(name)] = v;
  const { head, body } = input;
  const mirrored: TrackingInput = {
    blendshapes,
    head: { yaw: -head.yaw, pitch: head.pitch, roll: -head.roll },
  };
  if (body) {
    mirrored.body = {
      yaw: body.yaw === undefined ? undefined : -body.yaw,
      pitch: body.pitch,
      roll: body.roll === undefined ? undefined : -body.roll,
      shoulderLeft: body.shoulderRight,
      shoulderRight: body.shoulderLeft,
    };
  }
  return mirrored;
}

/** `WebPuppet.set_tracking` の引数の形にする。 */
export function trackingArguments(
  input: TrackingInput,
): [
  string[],
  Float32Array,
  number,
  number,
  number,
  number | undefined,
  number | undefined,
  number | undefined,
  number | undefined,
  number | undefined,
] {
  const names = Object.keys(input.blendshapes);
  const scores = Float32Array.from(names, (n) => input.blendshapes[n] ?? 0);
  const { head, body } = input;
  return [
    names,
    scores,
    head.yaw,
    head.pitch,
    head.roll,
    body?.yaw,
    body?.pitch,
    body?.roll,
    body?.shoulderLeft,
    body?.shoulderRight,
  ];
}
