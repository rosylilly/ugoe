// @ugoe/runtime: ugoe のランタイムを Web で使うための入口。
//
// - `WebPuppet`・`MotionSampler` などは ugoe-wasm (crates/ugoe-wasm) の JS バインディングそのもの。使い方は crates/ugoe-wasm/README.md。
// - ここでは初期化 (`initRuntime`)・作成 (`createPuppet`)・モデルの読み込み (`loadModel`) の薄い補助と、
//   Web Component の `<ugoe-avatar>` (`./element.ts`)、フェイストラッキングの入力の変換 (`./tracking.ts`) と
//   平滑化・キャリブレーション (`./tracking-filter.ts`)、読み込みの警告と着せ替えの例外の型 (`./warnings.ts`)、
//   読み込みの上限 (`./limits.ts`)、作成の例外の型 (`./create-errors.ts`)、描画と読み戻しの例外の型 (`./render-errors.ts`)、
//   設定の差し替え (`set_effects_json`・`set_physics_json`・`set_base_physics_json`) の例外の型 (`./settings-errors.ts`)、
//   問い合わせの JSON の型と読む補助 (`./queries.ts`) を足す。
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
  type UgoeAvatarElementEventMap,
  type UgoeAvatarErrorDetail,
  type UgoeAvatarLoadDetail,
} from "./avatar.js";
export {
  CREATE_ERROR_CODES,
  isCreateError,
  type UgoeCreateError,
  type UgoeCreateErrorCode,
} from "./create-errors.js";
export {
  isLimitError,
  isLoadError,
  LOAD_ERROR_CODES,
  type LoadLimits,
  limitsJson,
  type UgoeLimitError,
  type UgoeLimitErrorCode,
  type UgoeLimitErrorFields,
  type UgoeLoadError,
  type UgoeLoadErrorCode,
  type UgoeLoadErrorFields,
} from "./limits.js";
export {
  avatarSlots,
  currentPose,
  drawableStates,
  expressionInfos,
  moduleInfos,
  motionEvents,
  motionInfos,
  parameterDefinitions,
  slotDefinitions,
  type UgoeCurveTarget,
  type UgoeCurveValue,
  type UgoeDrawableState,
  type UgoeExpressionInfo,
  type UgoeExpressionParameter,
  type UgoeModuleInfo,
  type UgoeModuleRequirements,
  type UgoeMotionEvent,
  type UgoeMotionInfo,
  type UgoeParameterDefinition,
  type UgoeParameterDefinitions,
  type UgoeParameterGroup,
  type UgoePose,
  type UgoePoseEntry,
  type UgoeSlot,
  visiblePoseParts,
} from "./queries.js";
export {
  isRenderError,
  RENDER_ERROR_CODES,
  type UgoeRenderError,
  type UgoeRenderErrorCode,
} from "./render-errors.js";
export {
  type CreatePuppetOptions,
  createPuppet,
  fetchModel,
  type InitOptions,
  initRuntime,
  type LoadModelOptions,
  loadModel,
  sampleMotion,
} from "./runtime.js";
export {
  isSettingsError,
  SETTINGS_ERROR_CODES,
  type UgoeSettingsError,
  type UgoeSettingsErrorCode,
  type UgoeSettingsErrorFields,
} from "./settings-errors.js";
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
export {
  DRESS_UP_ERROR_CODES,
  detachedModules,
  isDressUpError,
  isKnownLoadWarning,
  LOAD_WARNING_CODES,
  loadWarnings,
  type UgoeDressUpError,
  type UgoeDressUpErrorCode,
  type UgoeDressUpErrorFields,
  type UgoeDressUpErrorInfo,
  type UgoeKnownLoadWarning,
  type UgoeLoadWarning,
  type UgoeModuleDetachedWarning,
  type UgoePhysicsRemovedWarning,
  type UgoeTextureLoadWarning,
  type UgoeUnknownLoadWarning,
  type UgoeUnknownOptionalFeatureWarning,
  type UgoeUnknownValueWarning,
} from "./warnings.js";
export {
  type InitInput,
  MotionSampler,
  sample_motion_json,
  version,
  WebPuppet,
} from "./wasm/ugoe_wasm.js";
