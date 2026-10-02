// バックエンドのエラー・警告 (`Message`。コード・引数・英語の文。ADR 0029、docs/editor.md#エラーと警告のコード)
// を画面で扱うための関数。IPC コマンドは失敗すると `Message` で reject する。
// 訳は T0663〜T0665 でカタログの `errors.<code>` から引く。いまは英語の文 (`message`) を出す。
import type { Message } from "./generated/types";

/** `value` がバックエンドの `Message` (`code` と `message` を持つオブジェクト) か。 */
export function isMessage(value: unknown): value is Message {
  if (value === null || typeof value !== "object") return false;
  const fields = value as Record<string, unknown>;
  return typeof fields.code === "string" && typeof fields.message === "string";
}

/** エラーのコード (`project.not_a_project` など)。`Message` でなければ `null`。 */
export function errorCode(error: unknown): string | null {
  return isMessage(error) ? error.code : null;
}

/** 画面に出すエラーの文。`Message` なら英語の文、`Error` ならその文、それ以外は文字列にしたもの。 */
export function errorText(error: unknown): string {
  if (isMessage(error)) return error.message;
  if (error instanceof Error) return error.message;
  return String(error);
}
