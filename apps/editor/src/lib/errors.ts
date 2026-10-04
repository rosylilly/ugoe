// バックエンドのエラー・警告 (`Message`。コード・引数・英語の文。ADR 0029、docs/editor.md#エラーと警告のコード)
// を画面で扱うための関数。IPC コマンドは失敗すると `Message` で reject する。
// 画面の文はカタログの `errors.<code>` を `args` で引く。カタログに無いコード (新しいバックエンドと古い画面など) は英語の文 (`message`) を出す。
import { hasTranslation } from "@ugoe/i18n";
import type { Message, MessageArg } from "./generated/types";
import { t } from "./i18n";

/** キーを組み立てて引く `t()` (コードと引数の値はカタログの型の外)。 */
const lookup = t as unknown as (key: string, options?: Record<string, unknown>) => string;

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

/** 画面に出すエラーの文。`Message` なら今の言語の文、`Error` ならその文、それ以外は文字列にしたもの。 */
export function errorText(error: unknown): string {
  if (isMessage(error)) return messageText(error);
  if (error instanceof Error) return error.message;
  return String(error);
}

/** カタログにコードの訳があるか (複数形のキーは `_other` で確かめる)。 */
function translatable(code: string): boolean {
  const key = `errors.${code}`;
  return hasTranslation("editor", key) || hasTranslation("editor", `${key}_other`);
}

/** 値が英語の識別子 (要素の種類など) になる引数。値はカタログの `errorArgs.<値>` で訳す。 */
const IDENTIFIER_ARGS = new Set(["kind", "what", "expected", "actual", "target_kind", "template"]);

/**
 * 引数の値。要素の種類などの英語の識別子 (`kind` の `mesh` など) は、カタログの `errorArgs.<値>` があれば訳す。
 * パス・名前・ID・自由な文 (`detail`・`reason`) はそのまま。
 */
function argText(name: string, value: MessageArg): MessageArg {
  if (typeof value !== "string" || !IDENTIFIER_ARGS.has(name)) return value;
  const key = `errorArgs.${value}`;
  return hasTranslation("editor", key) ? lookup(key) : value;
}

/** 一覧の中身 (`items`) の訳を `; ` でつないだ文。全件 (`count`) が項目より多ければ残りの件数を足す。 */
function itemsText(message: Message, items: Message[]): string {
  const texts = items.map(messageText);
  const count = message.args.count;
  const rest = typeof count === "number" ? count - items.length : 0;
  if (rest > 0) texts.push(t("errorListMore", { count: rest }));
  return texts.join("; ");
}

/**
 * `Message` (エラー・警告・問題) の今の言語の文。カタログに無いコードは英語の文 (`message`。原因の文も含む) をそのまま出す。
 *
 * - 原因 (`cause`) は、引数 `detail` を持つコードなら原因の訳を `detail` の代わりに差し込み、そうでなければ原因の文を後ろに付ける。
 * - 一覧の中身 (`items`) は、各項目の訳を `details` の代わりに差し込む (T0163)。
 */
export function messageText(message: Message): string {
  if (!translatable(message.code)) return message.message;
  const args: Record<string, MessageArg> = Object.fromEntries(
    Object.entries(message.args).map(([name, value]) => [name, argText(name, value)]),
  );
  let cause = message.cause;
  if (cause && "detail" in args) {
    args.detail = messageText(cause);
    cause = undefined;
  }
  if (message.items && message.items.length > 0 && "details" in args) {
    args.details = itemsText(message, message.items);
  }
  const text = lookup(`errors.${message.code}`, args);
  return cause ? t("errorCause", { message: text, cause: messageText(cause) }) : text;
}
