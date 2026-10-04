// バックエンドのコマンドのエラー (ADR 0029 の `code`・`args`・`message`。src-tauri/src/error.rs) を画面の文にする。
// 画面の文はカタログの `errors.<code>` を `args` で引く。カタログに無いコード (新しいバックエンドと古い画面など) は
// 英語の文 (`message`) を出す (エディタ・ビューアの lib/errors.ts と同じ扱い)。
//
// コマンドは reject した値をそのまま投げる (`host.ts`)。操作ウィンドウはコードで出し方を選ぶ (シーンのエラーはそのシーンに
// 出すなど) ので、ここで `Error` に替えない。操作ウィンドウ (ホットキーの知らせを含む) は `describeError` で今の言語の文にする。
// 文言を出さない出力ウィンドウとブラウザソースは、同じ形を lib/commandMessage.ts の `englishErrorText` で英語の文にする。
import { formatNumber, hasTranslation } from "@ugoe/i18n";
import { type CommandMessage, isCommandMessage } from "./commandMessage";
import { t } from "./i18n";

export { type CommandMessage, isCommandMessage };

/** キーを組み立てて引く `t()` (コードと引数の値はカタログの型の外)。 */
const lookup = t as unknown as (key: string, options?: Record<string, unknown>) => string;

/** コマンドのエラーの今の言語の文。数の引数 (バイト数など) は今の言語の書式にする。 */
export function messageText(error: CommandMessage): string {
  const key = `errors.${error.code}`;
  if (!hasTranslation("studio", key)) return error.message;
  const args = Object.fromEntries(
    Object.entries(error.args ?? {}).map(([name, value]) => [
      name,
      typeof value === "number" ? formatNumber(value) : value,
    ]),
  );
  return lookup(key, args);
}

/** 画面に出すエラーの文。コマンドのエラーなら今の言語の文、`Error` ならその文、それ以外は文字列にしたもの。 */
export function describeError(e: unknown): string {
  if (isCommandMessage(e)) return messageText(e);
  if (e instanceof Error) return e.message;
  return String(e);
}

/** シーンを出せないエラー (stage.rs の `SceneError` から作ったもの。コードが `scene.` で始まる)。 */
export function isSceneError(e: unknown): e is CommandMessage {
  return isCommandMessage(e) && e.code.startsWith("scene.");
}
