// 読み込みの上限 (T0152) と、上限を超えたときの例外の型。読み込みの失敗 (`LoadError`。T0434) の型もここに置く。
//
// 上限はファイル形式の決まりではなく、組み込む側の方針 (docs/spec/file-formats.md#読み込みの上限)。
// `loadModel`・`fetchModel`・`<ugoe-avatar>` の `loadLimits` で指定し、`WebPuppet` の `load_bundle(bytes, limitsJson)`・
// `load_module(bytes, limitsJson)` に渡す。`code` と補足の値は crates/ugoe-wasm/README.md#読み込みの上限-t0152。

/** 読み込みの上限。書かなかった上限は既定値。 */
export interface LoadLimits {
  /** 圧縮したチャンク 1 つを展開した後の大きさの上限 (バイト)。既定は 256 MiB。 */
  maxChunkDecompressed?: number;
  /** 1 つのファイルで、圧縮したチャンクを展開したバイトの合計の上限。既定は 1 GiB。 */
  maxTotalDecompressed?: number;
  /** デコードする PNG の幅と高さそれぞれの上限 (画素)。既定は 16384。 */
  maxImageSide?: number;
}

/**
 * `WebPuppet.load_bundle` / `load_module` に渡す上限の JSON。`limits` が無ければ `undefined` (既定値で読む)。
 * 値は 0 以上の整数 (違えば wasm が `invalid limits` の例外を投げる)。
 */
export function limitsJson(limits: LoadLimits | undefined): string | undefined {
  if (limits === undefined) return undefined;
  return JSON.stringify({
    max_chunk_decompressed: limits.maxChunkDecompressed,
    max_total_decompressed: limits.maxTotalDecompressed,
    max_image_side: limits.maxImageSide,
  });
}

/** 上限を超えたときの種類 (`code`)。 */
export type UgoeLimitErrorCode = "read_limit_exceeded" | "image_limit_exceeded";

/** 上限を超えたときの補足の値。種類に無い値は `undefined`。 */
export interface UgoeLimitErrorFields {
  /** `read_limit_exceeded`: どの上限か。 */
  limit?: "chunk_decompressed" | "total_decompressed" | (string & {});
  /** `read_limit_exceeded`: 展開をやめたチャンクのタグ (`"TEXR"` など)。 */
  chunk?: string;
  /** 超えた上限の値 (`read_limit_exceeded` はバイト、`image_limit_exceeded` は画素)。 */
  max?: number;
  /** `image_limit_exceeded`: テクスチャか法線マップか。 */
  image?: "texture" | "normal_map" | (string & {});
  /** `image_limit_exceeded`: テクスチャの番号。 */
  texture?: number;
  /** `image_limit_exceeded`: 画像の幅 (画素)。 */
  width?: number;
  /** `image_limit_exceeded`: 画像の高さ (画素)。 */
  height?: number;
}

/**
 * 読み込みの上限を超えた例外。`load_bundle` (`loadModel`) は `name` が `"LoadError"`、`load_module` は
 * `"DressUpError"` で投げる。`message` は英語の文で、安定した値ではない。
 */
export interface UgoeLimitError extends Error, UgoeLimitErrorFields {
  name: "LoadError" | "DressUpError";
  code: UgoeLimitErrorCode;
}

const LIMIT_CODES: readonly string[] = ["read_limit_exceeded", "image_limit_exceeded"];

/** `e` が読み込みの上限を超えた例外か。 */
export function isLimitError(e: unknown): e is UgoeLimitError {
  if (!(e instanceof Error)) return false;
  if (e.name !== "LoadError" && e.name !== "DressUpError") return false;
  const code = (e as { code?: unknown }).code;
  return typeof code === "string" && LIMIT_CODES.includes(code);
}

/**
 * このパッケージが知っている読み込みの失敗の種類 (`LoadError` の `code`)。意味は crates/ugoe-wasm/README.md#読み込みの失敗-t0434、
 * 一覧は crates/ugoe-wasm/codes.json の `load_error` と同じ (`codes.test.ts` が比べる)。
 */
export const LOAD_ERROR_CODES = [
  "invalid_file",
  "unsupported_version",
  "unsupported_features",
  "invalid_model",
  "compile",
  "read_limit_exceeded",
  "image_limit_exceeded",
  "invalid_texture",
  "texture_too_large",
  "texture",
  "device_lost",
] as const;

/** 読み込み (`load_json` / `load_bundle`。`loadModel`・`fetchModel`) の失敗の種類 (`LoadError` の `code`)。 */
export type UgoeLoadErrorCode = (typeof LOAD_ERROR_CODES)[number];

/** 読み込みの失敗の補足の値 (種類によって、あるものだけを持つ)。上限を超えたときの値は `UgoeLimitErrorFields`。 */
export interface UgoeLoadErrorFields extends UgoeLimitErrorFields {
  /** `unsupported_version`: ファイルの版。 */
  found?: number;
  /** `unsupported_version`: 読める版。 */
  supported?: number;
  /** `unsupported_features`: 知らない必須の機能の ID を `", "` でつないだもの。 */
  features?: string;
  /** `invalid_model`: 検査のエラーの数。 */
  errors?: number;
  /**
   * `invalid_model`: 検査のエラーの種類 (`ValidationError::code`) を出た順に、重なりを除いて `, ` でつないだ文字列
   * (着せ替えの `invalid_physics` の `error_codes` と同じ形。T0469)。個々のエラーの中身は `message` にだけある。
   */
  error_codes?: string;
}

/**
 * 読み込みの失敗 (`name` が `"LoadError"`)。`isLoadError` で見分ける。`code` は知らない値 (新しい版のランタイムの種類) も
 * ありうる。`message` は英語の文で、安定した値ではない。`texture_too_large` の `max` はデバイスの一辺の上限 (画素)。
 */
export interface UgoeLoadError extends Error, UgoeLoadErrorFields {
  name: "LoadError";
  code: UgoeLoadErrorCode | (string & {});
}

/** `e` が読み込みの失敗 (`LoadError`) か。 */
export function isLoadError(e: unknown): e is UgoeLoadError {
  return (
    e instanceof Error &&
    e.name === "LoadError" &&
    typeof (e as { code?: unknown }).code === "string"
  );
}
