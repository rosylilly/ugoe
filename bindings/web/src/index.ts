// @ugoe/runtime: ugoe のランタイムを Web で使うための入口。
//
// - `WebPuppet` などは ugoe-wasm (crates/ugoe-wasm) の JS バインディングそのもの。使い方は crates/ugoe-wasm/README.md。
// - ここでは初期化 (`initRuntime`)・作成 (`createPuppet`)・モデルの読み込み (`loadModel`) の薄い補助と、
//   Web Component の `<ugoe-avatar>` (`./element.ts`)、フェイストラッキングの入力の変換 (`./tracking.ts`) と
//   平滑化・キャリブレーション (`./tracking-filter.ts`) を足す。
// - 配布の方針は `docs/adr/0017-single-runtime-distribution.md`、ライセンスと置き場所は
//   `docs/adr/0025-web-runtime-package.md`。

export {
  type Backend,
  formatParameters,
  isBundle,
  type MotionPriority,
  parseParameters,
} from "./attributes.js";
export {
  defineUgoeAvatar,
  UgoeAvatarElement,
  type UgoeAvatarLoadDetail,
  type UgoeParameterDefinition,
} from "./avatar.js";
export {
  type CreatePuppetOptions,
  createPuppet,
  fetchModel,
  type InitOptions,
  initRuntime,
  loadModel,
} from "./runtime.js";
export {
  type FaceLandmarkerOptions,
  type FaceLandmarkerResultLike,
  headRotationFromMatrix,
  mirrorTracking,
  type TrackingBody,
  type TrackingInput,
  type TrackingRotation,
  trackingArguments,
  trackingFromFaceLandmarker,
} from "./tracking.js";
export {
  calibrateTracking,
  captureNeutral,
  type OneEuroOptions,
  TrackingFilter,
  type TrackingFilterOptions,
  type TrackingNeutral,
} from "./tracking-filter.js";
export { type InitInput, sample_motion_json, version, WebPuppet } from "./wasm/ugoe_wasm.js";
