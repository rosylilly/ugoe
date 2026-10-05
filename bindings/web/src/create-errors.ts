// `WebPuppet.create` の失敗のうち、コードを持つもの (`CreateError`。T0475) の型。
// `code` の意味は crates/ugoe-wasm/README.md#作成の失敗-t0475。

/**
 * このパッケージが知っている作成の失敗の種類 (`CreateError` の `code`)。一覧は crates/ugoe-wasm/codes.json の
 * `create_error` と同じ (`codes.test.ts` が比べる)。
 */
export const CREATE_ERROR_CODES = [
  "backend_unavailable",
  "context_unavailable",
  "adapter_unavailable",
  "device_unavailable",
] as const;

/** 作成の失敗の種類 (`CreateError` の `code`)。 */
export type UgoeCreateErrorCode = (typeof CREATE_ERROR_CODES)[number];

/**
 * `WebPuppet.create` の失敗 (`name` が `"CreateError"`)。`isCreateError` で見分ける。どれも GPU やブラウザの都合で
 * 作れなかったもので、新しい canvas で (時間を置いて) 作り直せば作れることがある。`code` は知らない値
 * (新しい版のランタイムの種類) もありうる。`message` は英語の文で、安定した値ではない。
 * 引数の誤り (知らない `backend`) などは、`code` を持たない `Error`。
 */
export interface UgoeCreateError extends Error {
  name: "CreateError";
  code: UgoeCreateErrorCode | (string & {});
}

/** `e` が作成の失敗 (`CreateError`) か。 */
export function isCreateError(e: unknown): e is UgoeCreateError {
  return (
    e instanceof Error &&
    e.name === "CreateError" &&
    typeof (e as { code?: unknown }).code === "string"
  );
}
