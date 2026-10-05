// `WebPuppet` の問い合わせ (`*_json()`) の戻り値の型と、それを読む補助 (T0452)。
//
// 形は crates/ugoe-wasm/README.md の各節と、`crates/ugoe-wasm/src` の doc コメントが決める。モデルの JSON 形式と同じ要素
// (パラメータ・スロット・ポーズ) は spec/model.md と spec/schema/model.schema.json の形。新しい版のランタイムは要素に
// キーを足しうるので、知らないキーがあっても読めるように使うこと。
// 読み込みの警告 (`load_warnings_json`・`detached_modules_json`) は `./warnings.ts`。物理 (`physics_json`・`base_physics_json`) は
// 型を出さない (bindings/web/README.md#問い合わせの-json-の型-t0452)。

import type { UgoeLoadWarning } from "./warnings.js";
import type { WebPuppet } from "./wasm/ugoe_wasm.js";

/** モデルのパラメータの定義 (`WebPuppet.parameters_json()` の `parameters` の要素。モデルの JSON 形式と同じ)。 */
export interface UgoeParameterDefinition {
  id: string;
  name: string;
  /** 中継用 (内部) のパラメータ (docs/spec/model.md#パラメータ)。`false` なら無い。 */
  internal?: boolean;
  min: number;
  max: number;
  default: number;
  /** `"normal"` か `"blend_shape"` (ブレンドシェイプ)。 */
  kind?: "normal" | "blend_shape" | (string & {});
  /** 左右ペアのパラメータの ID。 */
  mirror?: string | null;
  /** 所属するグループの ID。 */
  group?: string | null;
  [key: string]: unknown;
}

/** パラメータのグループ (エディタ・ビューアの表示用)。 */
export interface UgoeParameterGroup {
  id: string;
  name: string;
}

/** `WebPuppet.parameters_json()` の形。モデルが無ければ両方とも空。 */
export interface UgoeParameterDefinitions {
  parameters: UgoeParameterDefinition[];
  groups: UgoeParameterGroup[];
}

/** パラメータとグループの定義 (`parameters_json()`)。 */
export function parameterDefinitions(puppet: WebPuppet): UgoeParameterDefinitions {
  return JSON.parse(puppet.parameters_json()) as UgoeParameterDefinitions;
}

/** 読み込んだバンドルのモーション (`motions_json()` の要素。T0720)。 */
export interface UgoeMotionInfo {
  name: string;
  /** 長さ (秒)。 */
  duration: number;
  looping: boolean;
  /** フェードイン・フェードアウトの時間 (秒)。 */
  fade_in: number;
  fade_out: number;
  /** カーブが動かすパラメータの ID (重なりなし、カーブの順)。 */
  parameters: string[];
  /** イベントの値 (時刻の順)。 */
  events: string[];
}

/** 読み込んだバンドルのモーションの一覧 (`motions_json()`。チャンクの順)。 */
export function motionInfos(puppet: WebPuppet): UgoeMotionInfo[] {
  return JSON.parse(puppet.motions_json()) as UgoeMotionInfo[];
}

/** 直前の `update` で発火したモーションのイベント (`motion_events_json()` の要素。T0462)。 */
export interface UgoeMotionEvent {
  /** イベントの値。 */
  value: string;
  /** モーションの中の時刻 (秒)。 */
  time: number;
  /** イベントを発火したモーションの名前 (クロスフェードでフェードアウト中のモーションのこともある)。 */
  motion: string;
  /**
   * イベントを発火したモーションを始めた番号 (この `WebPuppet` で開始したモーションに 1 から振る。開始しなかった要求では進まない)。
   * 同じ名前のモーションを始め直したものを見分けるのに使う。
   */
  number: number;
}

/**
 * 直前の `update` で発火したモーションのイベントと、それを発火したモーション (`motion_events_json()`。T0462)。
 * 開始した順のモーションごとに、発火した順。値だけなら `puppet.motion_events()`。
 */
export function motionEvents(puppet: WebPuppet): UgoeMotionEvent[] {
  return JSON.parse(puppet.motion_events_json()) as UgoeMotionEvent[];
}

/** 表情のパラメータの値と合成の方法 (`*.expression.json` の `parameters` の要素)。 */
export interface UgoeExpressionParameter {
  parameter: string;
  value: number;
  /** 未知の `op` の項目 (新しい版のファイル) は一覧に出ない。 */
  op: "add" | "multiply" | "overwrite";
}

/** 読み込んだバンドルの表情 (`expressions_json()` の要素。`*.expression.json` の形。T0720)。 */
export interface UgoeExpressionInfo {
  name: string;
  /** フェードイン・フェードアウトの時間 (秒)。 */
  fade_in: number;
  fade_out: number;
  parameters: UgoeExpressionParameter[];
}

/** 読み込んだバンドルの表情の一覧 (`expressions_json()`。チャンクの順)。 */
export function expressionInfos(puppet: WebPuppet): UgoeExpressionInfo[] {
  return JSON.parse(puppet.expressions_json()) as UgoeExpressionInfo[];
}

/** モジュールが求める本体の条件 (`.ugoemod` の `requires`)。書かれていない条件は無い。 */
export interface UgoeModuleRequirements {
  /** 本体に求めるアバタープロファイルのレベル。 */
  avatar_level?: number;
  /** 本体に必要なスロットの ID。 */
  slots?: string[];
  /** 本体に必要なパラメータの ID。 */
  parameters?: string[];
}

/** 読み込んだモジュール (`modules_json()` の要素。T0123)。 */
export interface UgoeModuleInfo {
  name: string;
  /** 付けるスロットの ID。 */
  slot: string;
  requires: UgoeModuleRequirements;
  /** 今付けているか。 */
  attached: boolean;
  /** テクスチャの数。 */
  textures: number;
  /** 読み込みの警告の文 (英語。安定した値ではない)。 */
  warnings: string[];
  /** `warnings` と同じ順の構造 (`load_warnings_json()` の要素と同じ形)。 */
  warning_details: UgoeLoadWarning[];
}

/** 読み込んだモジュールの一覧 (`modules_json()`。読み込んだ順)。 */
export function moduleInfos(puppet: WebPuppet): UgoeModuleInfo[] {
  return JSON.parse(puppet.modules_json()) as UgoeModuleInfo[];
}

/**
 * 本体のスロット (`slots_json()` の要素。モデルの JSON 形式と同じ)。空の一覧と無い値はキーごと省かれる。
 * 意味は docs/spec/modules.md#スロット。
 */
export interface UgoeSlot {
  id: string;
  /** 取り付け先のデフォーマの ID。無ければキャンバス。 */
  anchor?: string;
  /** このスロットに入るメッシュの描画順の範囲。 */
  draw_order: { min: number; max: number };
  /** モジュールを付けたときに隠す本体のパーツの ID。 */
  replaces?: string[];
  /** モジュールが使ってよい本体のパラメータの ID。 */
  parameters?: string[];
  /** モジュールがマスクに使ってよい本体のメッシュの ID。 */
  mask_sources?: string[];
  /** モジュールの最上位のパーツの親にする本体のパーツの ID。 */
  part?: string;
  /** モジュールのリグが衝突してよい本体の衝突体の ID。 */
  colliders?: string[];
  /** モジュールのパーツを追従させてよい、本体のポーズの要素のパーツの ID。 */
  pose_parts?: string[];
  [key: string]: unknown;
}

/** 本体のスロット (`slots_json()`。モジュールを付けていても本体のもの)。モデルが無ければ `[]`。 */
export function slotDefinitions(puppet: WebPuppet): UgoeSlot[] {
  return JSON.parse(puppet.slots_json()) as UgoeSlot[];
}

/**
 * 本体のアバタープロファイルがアプリに公開するスロットの ID (`avatar_slots_json()`。T0715)。
 * プロファイルかモデルが無ければ `null` (公開の区別が無い)。公開していないスロットは利用者に見せないことが推奨される。
 */
export function avatarSlots(puppet: WebPuppet): string[] | null {
  return JSON.parse(puppet.avatar_slots_json()) as string[] | null;
}

/** 要素 1 つの評価の結果 (`drawable_states_json()` の要素。docs/spec/evaluation.md#8-出力)。 */
export interface UgoeDrawableState {
  opacity: number;
  draw_order: number;
  /** 乗算色 `[r, g, b]`。 */
  multiply_color: [number, number, number];
  /** スクリーン色 `[r, g, b]`。 */
  screen_color: [number, number, number];
  mask_strength: number;
  visible: boolean;
}

/** 要素ごとの評価の結果 (`drawable_states_json()`。`drawable_ids()` の順)。モデルが無ければ `[]`。 */
export function drawableStates(puppet: WebPuppet): UgoeDrawableState[] {
  return JSON.parse(puppet.drawable_states_json()) as UgoeDrawableState[];
}

/** ポーズの要素 (グループの中で排他的に表示するパーツと、それに追従するパーツ)。 */
export interface UgoePoseEntry {
  part: string;
  links: string[];
}

/** 今のポーズ (`pose_json()`。モデルの `pose` の形。着せ替えで付けたモジュールのグループと追従も入る)。 */
export interface UgoePose {
  /** 切り替えのクロスフェードの時間 (秒)。 */
  fade_duration: number;
  groups: { entries: UgoePoseEntry[] }[];
}

/** 今のポーズ (`pose_json()`)。モデルが無ければ `undefined`。 */
export function currentPose(puppet: WebPuppet): UgoePose | undefined {
  const json = puppet.pose_json();
  return json === undefined ? undefined : (JSON.parse(json) as UgoePose);
}

/**
 * 表示中のポーズの要素のパーツの ID (`pose_visible_json()`。T0478)。`currentPose` の `groups` と同じ並びでグループごとに 1 つ、
 * 要素の無いグループは `null`。`show_pose` で選んだ要素はすぐに、モーションのパーツの不透明度のカーブの要求で切り替わった要素は
 * 次の `update` の後に入る。モデルが無ければ `undefined`。
 */
export function visiblePoseParts(puppet: WebPuppet): (string | null)[] | undefined {
  const json = puppet.pose_visible_json();
  return json === undefined ? undefined : (JSON.parse(json) as (string | null)[]);
}

/** カーブの対象 (`*.motion.json` の `target`)。 */
export interface UgoeCurveTarget {
  kind: "parameter" | "part_opacity" | "effect_group";
  /** パラメータ・パーツの ID か、エフェクトグループの名前。 */
  id: string;
}

/** カーブ 1 つの値 (`sample_motion_json`・`MotionSampler.sample` の要素。`sampleMotion` (`./runtime.ts`) で読む)。 */
export interface UgoeCurveValue {
  target: UgoeCurveTarget;
  value: number;
}
