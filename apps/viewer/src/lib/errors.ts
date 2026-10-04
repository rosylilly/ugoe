// バックエンドのコマンドのエラー (ADR 0029 の `code`・`args`・`message`。src-tauri/src/error.rs) を画面の文にする。
// 画面の文はカタログの `errors.<code>` を `args` で引く。カタログに無いコード (新しいバックエンドと古い画面など) は
// 英語の文 (`message`) を出す (エディタの lib/errors.ts と同じ扱い)。
//
// コマンドは `host.ts` から呼び、失敗はそこで `CommandError` (`Error`。`message` は今の言語の文) に替えて投げる。
// 受け取る側 (`@ugoe/player`・`@ugoe/face-tracking`・`drop.ts` など) は、ほかの例外と同じく `Error` の文を出せばよい。
import { formatNumber, hasTranslation } from "@ugoe/i18n";
import { t } from "./i18n";

/** コマンドのエラー (バックエンドが reject する値)。 */
export interface CommandMessage {
  /** 安定した識別子 (`file.too_large` など。README の「コマンドのエラー」)。 */
  code: string;
  /** 文に差し込む値 (パス・バイト数など)。 */
  args: Record<string, string | number>;
  /** 英語の文 (カタログに無いコードのとき出す)。 */
  message: string;
}

/** キーを組み立てて引く `t()` (コードと引数の値はカタログの型の外)。 */
const lookup = t as unknown as (key: string, options?: Record<string, unknown>) => string;

/** `value` がコマンドのエラー (`code` と `message` を持つオブジェクト) か。 */
export function isCommandMessage(value: unknown): value is CommandMessage {
  if (value === null || typeof value !== "object") return false;
  const fields = value as Record<string, unknown>;
  return typeof fields.code === "string" && typeof fields.message === "string";
}

/** コマンドのエラーの今の言語の文。数の引数 (バイト数など) は今の言語の書式にする。 */
export function messageText(error: CommandMessage): string {
  const key = `errors.${error.code}`;
  if (!hasTranslation("viewer", key)) return error.message;
  const args = Object.fromEntries(
    Object.entries(error.args ?? {}).map(([name, value]) => [
      name,
      typeof value === "number" ? formatNumber(value) : value,
    ]),
  );
  return lookup(key, args);
}

/** コマンドの失敗を表す例外。`message` は今の言語の文で、`code`・`args` はバックエンドのまま。 */
export class CommandError extends Error {
  readonly code: string;
  readonly args: Record<string, string | number>;

  constructor(error: CommandMessage) {
    super(messageText(error));
    this.name = "CommandError";
    this.code = error.code;
    this.args = error.args ?? {};
  }
}

/** コマンドが reject した値を例外にする (コマンドのエラーなら `CommandError`、ほかはそのまま)。 */
export function commandFailure(reason: unknown): unknown {
  return isCommandMessage(reason) ? new CommandError(reason) : reason;
}

/** 画面に出すエラーの文。コマンドのエラーなら今の言語の文、`Error` ならその文、それ以外は文字列にしたもの。 */
export function errorText(error: unknown): string {
  if (isCommandMessage(error)) return messageText(error);
  if (error instanceof Error) return error.message;
  return String(error);
}
