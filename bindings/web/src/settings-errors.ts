// 設定の差し替え (`set_effects_json`・`set_physics_json`・`set_base_physics_json`) の検査の失敗 (T0461・T0469) の型。
// `code` の意味は crates/ugoe-wasm/README.md#物理モーション表情エフェクト-t0309 の節と
// crates/ugoe-wasm/README.md#code-の一覧-t0452 (`settings_error`)。

/**
 * このパッケージが知っている設定の差し替えの検査の失敗の種類。一覧は crates/ugoe-wasm/codes.json の `settings_error` と同じ
 * (`codes.test.ts` が比べる)。
 *
 * - `invalid_model`: `set_effects_json` の設定が検査を通らない (読み込みの `invalid_model` と同じ)。
 * - `invalid_physics`: `set_physics_json`・`set_base_physics_json` の設定が検査を通らない (着せ替えの `invalid_physics` と同じ)。
 */
export const SETTINGS_ERROR_CODES = ["invalid_model", "invalid_physics"] as const;

/** 設定の差し替えの検査の失敗の種類。 */
export type UgoeSettingsErrorCode = (typeof SETTINGS_ERROR_CODES)[number];

/** 設定の差し替えの検査の失敗の補足の値 (読み込みの `invalid_model`・着せ替えの `invalid_physics` と同じ形)。 */
export interface UgoeSettingsErrorFields {
  /** 検査のエラーの数 (1 以上)。 */
  errors?: number;
  /**
   * 検査のエラーの種類 (`ValidationError::code`) を出た順に、重なりを除いて `, ` でつないだ文字列
   * (例: `"invalid_value, unknown_reference"`)。個々のエラーの中身は `message` にだけある。
   */
  error_codes?: string;
}

/**
 * `set_effects_json`・`set_physics_json`・`set_base_physics_json` の検査の失敗。`isSettingsError` で見分ける。
 * `name` は `"Error"` のまま (専用の名前を持たない)。`message` は英語の文で、安定した値ではない。
 * 読めない JSON などほかの失敗は `code` を持たない `Error`。
 */
export interface UgoeSettingsError extends Error, UgoeSettingsErrorFields {
  code: UgoeSettingsErrorCode;
}

const CODES: readonly string[] = SETTINGS_ERROR_CODES;

/**
 * `e` が設定の差し替えの検査の失敗か。`name` で見分けられないので、`code` がこのパッケージの知っている種類のものだけを受ける
 * (新しい版のランタイムが種類を足しても、ここでは受けない)。
 */
export function isSettingsError(e: unknown): e is UgoeSettingsError {
  if (!(e instanceof Error)) return false;
  const code = (e as { code?: unknown }).code;
  return typeof code === "string" && CODES.includes(code);
}
