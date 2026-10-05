// 描画と読み戻し (`render`・`read_pixels`) の失敗のうち、コードを持つもの (`RenderError`。T0465) の型。
// `code` の意味は crates/ugoe-wasm/README.md#描画と読み戻しの失敗-t0465。

/**
 * このパッケージが知っている描画と読み戻しの失敗の種類 (`RenderError` の `code`)。一覧は crates/ugoe-wasm/codes.json の
 * `render_error` と同じ (`codes.test.ts` が比べる)。
 */
export const RENDER_ERROR_CODES = ["device_lost"] as const;

/** 描画と読み戻しの失敗の種類 (`RenderError` の `code`)。 */
export type UgoeRenderErrorCode = (typeof RENDER_ERROR_CODES)[number];

/**
 * `render`・`read_pixels` の失敗 (`name` が `"RenderError"`)。`isRenderError` で見分ける。`code` は知らない値
 * (新しい版のランタイムの種類) もありうる。`message` は英語の文で、安定した値ではない。
 * ほかの描画の失敗 (組み込みの誤りや一時的なもの) は、`code` を持たない `Error`。
 */
export interface UgoeRenderError extends Error {
  name: "RenderError";
  code: UgoeRenderErrorCode | (string & {});
}

/** `e` が描画と読み戻しの失敗 (`RenderError`) か。 */
export function isRenderError(e: unknown): e is UgoeRenderError {
  return (
    e instanceof Error &&
    e.name === "RenderError" &&
    typeof (e as { code?: unknown }).code === "string"
  );
}
