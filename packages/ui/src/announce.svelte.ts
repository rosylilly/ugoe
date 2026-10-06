// 読み上げだけの知らせ (docs/ui.md#状態の変化の知らせ)。画面の文や Toast に出ない状態の変化 (シーンが変わった・
// 提案が届いた) を、スクリーンリーダーに渡す。領域は `<Toaster />` が描く (アプリの根元に 1 つ。描く前から置いておく
// live region でないと、読み上げない読み上げソフトがある)。

/** 今の知らせ (polite と assertive)。 */
const messages = $state({ polite: "", assertive: "" });

/** `announce()` の指定。 */
export interface AnnounceOptions {
  /**
   * 読んでいる途中でも割り込んで読ませるか (`aria-live="assertive"`)。既定は false (読み終わるのを待つ)。
   * 利用者がすぐ知るべき失敗だけに使う (失敗は画面にも Notice で出す)。
   */
  assertive?: boolean;
}

/**
 * 文を読み上げさせる。見た目には何も出ない。同じ文が続いても読まれるよう、前と同じ文なら末尾に
 * 見えない空白 (U+00A0) を足し引きして変える。翻訳を通した文を渡す。空の文は無視する。
 */
export function announce(message: string, options: AnnounceOptions = {}): void {
  const text = message.trim();
  if (text === "") return;
  const key = options.assertive ? "assertive" : "polite";
  messages[key] = messages[key] === text ? `${text} ` : text;
}

/** 今の知らせ (`<Toaster />` が描く)。 */
export function announcements(): Readonly<{ polite: string; assertive: string }> {
  return messages;
}

/** 知らせを空にする (テスト用)。 */
export function clearAnnouncements(): void {
  messages.polite = "";
  messages.assertive = "";
}
