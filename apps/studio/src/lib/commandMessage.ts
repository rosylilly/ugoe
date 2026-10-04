// バックエンドのコマンドのエラーの形 (ADR 0029 の `code`・`args`・`message`。src-tauri/src/error.rs)。
// 文言を出さない出力ウィンドウとブラウザソースも読むので、@ugoe/i18n を読まない (訳すのは lib/errors.ts)。

/** コマンドのエラー (バックエンドが reject する値。ホットキーの知らせの `error` も同じ形)。 */
export interface CommandMessage {
  /** 安定した識別子 (`profile.not_json` など。README の「コマンドのエラー」)。 */
  code: string;
  /** 文に差し込む値 (パス・バイト数など)。 */
  args: Record<string, string | number>;
  /** 英語の文 (ログ向け。カタログに無いコードのときは画面にも出す)。 */
  message: string;
}

/** `value` がコマンドのエラー (`code` と `message` を持つオブジェクト) か。 */
export function isCommandMessage(value: unknown): value is CommandMessage {
  if (value === null || typeof value !== "object") return false;
  const fields = value as Record<string, unknown>;
  return typeof fields.code === "string" && typeof fields.message === "string";
}

/** 訳さない (英語の) エラーの文。コマンドのエラーなら `message`、`Error` ならその文、それ以外は文字列にしたもの。 */
export function englishErrorText(e: unknown): string {
  if (isCommandMessage(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}
