// モデルの読み込み (`load_json` / `load_bundle`) の失敗 (`LoadError`。T0434) を、利用者に分かる言葉 (今の言語) にする。
// `code` と補足の値の一覧は crates/ugoe-wasm/README.md#読み込みの失敗-t0434。カタログは名前空間 `player` の `loadError`。
// 知らないコード (新しい wasm と古い画面など) と `LoadError` でない例外は、wasm の文 (英語) のまま出す。
import { formatNumber } from "@ugoe/i18n";
import { t } from "./i18n";

/** キーを組み立てて引く `t()` (キーと引数の値はカタログの型の外)。 */
const lookup = t as unknown as (key: string, options?: Record<string, unknown>) => string;

/** `code` ごとの文のキー。`normalMap` は `image` が `"normal_map"` のとき、`total` は合計の上限 (`limit` が `"total_decompressed"`) のときのキー。 */
const LOAD_ERROR_KEYS: Record<string, { key: string; normalMap?: string; total?: string }> = {
  invalid_file: { key: "loadError.invalidFile" },
  unsupported_version: { key: "loadError.unsupportedVersion" },
  unsupported_features: { key: "loadError.unsupportedFeatures" },
  invalid_model: { key: "loadError.invalidModel" },
  compile: { key: "loadError.compile" },
  read_limit_exceeded: {
    key: "loadError.readLimitExceeded",
    total: "loadError.readLimitExceededTotal",
  },
  image_limit_exceeded: {
    key: "loadError.imageLimitExceeded",
    normalMap: "loadError.imageLimitExceededNormalMap",
  },
  invalid_texture: {
    key: "loadError.invalidTexture",
    normalMap: "loadError.invalidTextureNormalMap",
  },
  texture_too_large: {
    key: "loadError.textureTooLarge",
    normalMap: "loadError.textureTooLargeNormalMap",
  },
  texture: { key: "loadError.texture" },
  device_lost: { key: "loadError.deviceLost" },
};

/** 読み込みの上限を超えたときのコード (`load_module` の `DressUpError` もこの 2 つを使う。T0152)。 */
export const LOAD_LIMIT_CODES: readonly string[] = ["read_limit_exceeded", "image_limit_exceeded"];

function record(v: unknown): Record<string, unknown> {
  return typeof v === "object" && v !== null ? (v as Record<string, unknown>) : {};
}

/** `e` の `code` (`name` が `names` のどれかの `Error` で、`code` が文字列のとき)。 */
export function errorCode(e: unknown, names: readonly string[]): string | null {
  if (!(e instanceof Error) || !names.includes(e.name)) return null;
  const code = record(e).code;
  return typeof code === "string" ? code : null;
}

/** `code` と補足の値から文を作る (知らないコードは `null`)。数の補足の値は今の言語の書式にする。 */
function describe(code: string, e: Error): string | null {
  const keys = LOAD_ERROR_KEYS[code];
  if (keys === undefined) return null;
  const fields = record(e);
  const key =
    (fields.image === "normal_map" ? keys.normalMap : undefined) ??
    (fields.limit === "total_decompressed" ? keys.total : undefined) ??
    keys.key;
  const args: Record<string, unknown> = { detail: e.message };
  for (const [name, value] of Object.entries(fields)) {
    if (typeof value === "number") args[name] = formatNumber(value);
    else if (typeof value === "string") args[name] = value;
  }
  return lookup(key, args);
}

/**
 * 読み込みの失敗を、今の言語の文にする。原因が細かいもの (壊れたファイル・検査のエラーなど) は、wasm の文 (英語) を
 * 括弧で添える (`detail`)。`LoadError` でない例外と知らないコードは、例外の文のまま。
 */
export function describeLoadError(e: unknown): string {
  const code = errorCode(e, ["LoadError"]);
  const text = code === null ? null : describe(code, e as Error);
  return text ?? (e instanceof Error ? e.message : String(e));
}

/**
 * `load_module` が読み込みの上限を超えた `DressUpError` (`read_limit_exceeded`・`image_limit_exceeded`) の文。
 * ほかの例外は `null` (着せ替えの文にする)。
 */
export function describeLimitError(e: unknown): string | null {
  const code = errorCode(e, ["LoadError", "DressUpError"]);
  if (code === null || !LOAD_LIMIT_CODES.includes(code)) return null;
  return describe(code, e as Error);
}

/** 描画と読み戻しの失敗 (`RenderError`) の `code` ごとの文のキー。GPU が止まったのは読み込みと同じ文。 */
const RENDER_ERROR_KEYS: Record<string, string> = {
  device_lost: "loadError.deviceLost",
};

/**
 * 描画と読み戻し (`render`・`read_pixels`) の失敗 (`RenderError`。T0465。コードの一覧は
 * crates/ugoe-wasm/README.md#描画と読み戻しの失敗-t0465) を、今の言語の文にする。`RenderError` でない例外と知らないコードは、例外の文のまま。
 */
export function describeRenderError(e: unknown): string {
  const code = errorCode(e, ["RenderError"]);
  const key = code === null ? undefined : RENDER_ERROR_KEYS[code];
  if (key !== undefined) return lookup(key, { detail: (e as Error).message });
  return e instanceof Error ? e.message : String(e);
}

/** 作成の失敗 (`CreateError`) の `code` ごとの文のキー。 */
const CREATE_ERROR_KEYS: Record<string, string> = {
  backend_unavailable: "createError.backendUnavailable",
  context_unavailable: "createError.contextUnavailable",
  adapter_unavailable: "createError.adapterUnavailable",
  device_unavailable: "createError.deviceUnavailable",
};

/**
 * `e` が作成の失敗 (`CreateError`。T0475) か。どれも GPU やブラウザの都合で、時間を置いて作り直せば作れることがある
 * (知らないコードも同じに扱う)。
 */
export function isCreateFailure(e: unknown): boolean {
  return errorCode(e, ["CreateError"]) !== null;
}

/**
 * `WebPuppet.create` の失敗 (`CreateError`。T0475。コードの一覧は crates/ugoe-wasm/README.md#作成の失敗-t0475) を、
 * 今の言語の文にする (「描画を始められません」から始まる 1 つの文)。`CreateError` でない例外と知らないコードは `null`。
 */
export function describeCreateError(e: unknown): string | null {
  const code = errorCode(e, ["CreateError"]);
  const key = code === null ? undefined : CREATE_ERROR_KEYS[code];
  return key === undefined ? null : lookup(key);
}
