// 読み込みの警告の構造 (`WebPuppet.load_warnings_json()`。T0125) と、着せ替えの `DressUpError` (T0124) の型と、それを読む補助。
//
// `code` と補足の値は SDK の `ugoe::LoadWarning`・`ugoe::DressUpError` が決める
// (crates/ugoe-wasm/README.md#読み込みの警告-t0125、#着せ替え-t0123)。新しい版のランタイムは知らない `code` を返しうるので、
// 型は知らない `code` も受ける。

import type { WebPuppet } from "./wasm/ugoe_wasm.js";

/** 着せ替えの失敗の種類 (`DressUpError` の `code`)。 */
export type UgoeDressUpErrorCode =
  | "unknown_slot"
  | "missing_slot"
  | "missing_parameter"
  | "parameter_not_exposed"
  | "avatar_level"
  | "slot_occupied"
  | "unknown_mask"
  | "unknown_collider"
  | "pose_not_exposed"
  | "texture_out_of_range"
  | "invalid_composition"
  | "invalid_module_name"
  | "duplicate_module"
  | "compile"
  | "invalid_physics"
  | "not_attached"
  | "not_loaded"
  | "no_model"
  | "invalid_file"
  | "texture";

/** 着せ替えの失敗の補足の値 (種類によって、あるものだけを持つ)。 */
export interface UgoeDressUpErrorFields {
  /** モジュールの名前。 */
  module?: string;
  /** スロットの ID。 */
  slot?: string;
  /** パラメータの ID。 */
  parameter?: string;
  /** メッシュの ID。 */
  mesh?: string;
  /** `slot_occupied` で、スロットに付いているモジュールの名前。 */
  attached?: string;
}

/**
 * 着せ替えの付け外し (`attach_module`・`set_modules` など) が投げる例外。`isDressUpError` で見分ける。
 * `code` は知らない値 (新しい版のランタイムの種類) もありうる。`message` は英語の文で、安定した値ではない。
 */
export interface UgoeDressUpError extends Error, UgoeDressUpErrorFields {
  name: "DressUpError";
  code: UgoeDressUpErrorCode | (string & {});
}

/** `e` が着せ替えの `DressUpError` (wasm-bindgen が投げた `Error`) か。 */
export function isDressUpError(e: unknown): e is UgoeDressUpError {
  return (
    e instanceof Error &&
    e.name === "DressUpError" &&
    typeof (e as { code?: unknown }).code === "string"
  );
}

/** `module_detached` の `error` (付けるときの `DressUpError` と同じ `code` と補足の値に、英語の文を添えたもの)。 */
export interface UgoeDressUpErrorInfo extends UgoeDressUpErrorFields {
  code: UgoeDressUpErrorCode | (string & {});
  /** 英語の文 (安定した値ではない)。 */
  message: string;
}

/** 読み込みの警告に共通の値。 */
interface LoadWarningBase {
  /** 読み込みの戻り値の配列の同じ位置の文 (英語) と同じ文字列。 */
  warning: string;
}

/** テクスチャを代用した (`texture_ktx2`・`texture_missing`) か、法線マップなしで描く (`normal_map_ktx2`)。 */
export interface UgoeTextureLoadWarning extends LoadWarningBase {
  code: "texture_ktx2" | "texture_missing" | "normal_map_ktx2";
  /** テクスチャの番号。 */
  texture: number;
}

/** `load_json` で読み直したとき、引き継いだ物理の設定が新しいモデルと合わないので外した。 */
export interface UgoePhysicsRemovedWarning extends LoadWarningBase {
  code: "physics_removed";
  /** 英語の理由。 */
  reason: string;
}

/** 付けていたモジュールを付け直せないので外した (`detached_modules_json()` の要素と同じ)。 */
export interface UgoeModuleDetachedWarning extends LoadWarningBase {
  code: "module_detached";
  module: string;
  slot: string;
  error: UgoeDressUpErrorInfo;
}

/** 知らない任意の機能を宣言したファイル (新しい版で書かれたもの) をフォールバックで読んだ。 */
export interface UgoeUnknownOptionalFeatureWarning extends LoadWarningBase {
  code: "unknown_optional_feature";
  /** 機能の ID。 */
  feature: string;
}

/** 知らない列挙の値をフォールバックで読んだ。 */
export interface UgoeUnknownValueWarning extends LoadWarningBase {
  code: "unknown_value";
  /** チャンクとその中の場所。 */
  path: string;
}

/** このパッケージが知っている種類の読み込みの警告。`code` で見分けられる。 */
export type UgoeKnownLoadWarning =
  | UgoeTextureLoadWarning
  | UgoePhysicsRemovedWarning
  | UgoeModuleDetachedWarning
  | UgoeUnknownOptionalFeatureWarning
  | UgoeUnknownValueWarning;

/** 知らない種類 (新しい版のランタイムが返したもの) の読み込みの警告。`warning` をそのまま出せばよい。 */
export interface UgoeUnknownLoadWarning extends LoadWarningBase {
  code: string;
  [key: string]: unknown;
}

/**
 * 読み込みの警告の構造 (`load_warnings_json()` の要素)。`isKnownLoadWarning` で知っている種類に絞ってから
 * `code` で見分ける (知らない種類を含むままでは、`code` で比べても補足の値の型が決まらないため)。
 */
export type UgoeLoadWarning = UgoeKnownLoadWarning | UgoeUnknownLoadWarning;

const KNOWN_LOAD_WARNING_CODES: ReadonlySet<string> = new Set<UgoeKnownLoadWarning["code"]>([
  "texture_ktx2",
  "texture_missing",
  "normal_map_ktx2",
  "physics_removed",
  "module_detached",
  "unknown_optional_feature",
  "unknown_value",
]);

/** `warning` がこのパッケージの知っている種類か。 */
export function isKnownLoadWarning(warning: UgoeLoadWarning): warning is UgoeKnownLoadWarning {
  return KNOWN_LOAD_WARNING_CODES.has(warning.code);
}

/**
 * 直前の読み込み (`load_json` / `load_bundle`) の警告の構造。読み込みの戻り値の文の警告と同じ順・同じ数に並ぶ。
 * 次の読み込みが成功するまで変わらない (無ければ `[]`)。
 */
export function loadWarnings(puppet: WebPuppet): UgoeLoadWarning[] {
  return JSON.parse(puppet.load_warnings_json()) as UgoeLoadWarning[];
}

/** 直前の読み込みで、付け直せずに外したモジュール (`detached_modules_json()`)。 */
export function detachedModules(puppet: WebPuppet): UgoeModuleDetachedWarning[] {
  return JSON.parse(puppet.detached_modules_json()) as UgoeModuleDetachedWarning[];
}
