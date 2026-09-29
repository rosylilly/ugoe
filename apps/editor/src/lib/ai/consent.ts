// 外部の AI サービスに送る前の同意の流れ (docs/editor.md#ai-の設定と送信の同意)。
// 送る IPC コマンドは、同意がなければ送らずに `needs_consent` を返す。`withConsent` はそのときに
// 確認のダイアログ (ConsentDialog.svelte) を出し、同意を付けて呼び直す。
import type { AiReply, ConsentAnswer, DisclosureView } from "../ipc";

/** 送る内容を示して利用者に尋ねる。送らないなら `null`。 */
export type AskConsent = (disclosure: DisclosureView) => Promise<ConsentAnswer | null>;

/**
 * 外部に送る IPC コマンドを、同意を得てから呼ぶ。`call` は同じ入力で 2 回まで呼ばれる
 * (1 回目は同意なし、確認が要れば 2 回目に同意を付けて)。利用者が送らないことを選んだら `null`。
 * 送信の失敗・プロジェクトが「外部に送らない」のときは `call` の例外がそのまま投げられる。
 */
export async function withConsent<T>(
  call: (consent: ConsentAnswer | null) => Promise<AiReply<T>>,
  ask: AskConsent,
): Promise<{ value: T } | null> {
  let reply = await call(null);
  if (reply.status === "needs_consent") {
    const answer = await ask(reply.disclosure);
    if (!answer) return null;
    reply = await call(answer);
    // 同意した内容と送る内容が食い違った (間に入力が変わった)。黙って送り直さない。
    if (reply.status === "needs_consent") {
      throw new Error("The data to send changed after you agreed. Try again.");
    }
  }
  return { value: reply.value };
}

/** バイト数を読みやすく (1 KB = 1024 B)。 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

/** 送り先の規約へのリンク。 */
export interface ProviderTerms {
  /** データを扱う組織の名前。 */
  org: string;
  links: readonly { label: string; url: string }[];
}

/** 送り先のホストごとの規約 (確認の画面に示す)。 */
const TERMS_BY_HOST: Record<string, ProviderTerms> = {
  "api.anthropic.com": {
    org: "Anthropic",
    links: [
      { label: "Commercial Terms", url: "https://www.anthropic.com/legal/commercial-terms" },
      { label: "Privacy Policy", url: "https://www.anthropic.com/legal/privacy" },
    ],
  },
  "api.openai.com": {
    org: "OpenAI",
    links: [
      { label: "Services Agreement", url: "https://openai.com/policies/services-agreement/" },
      { label: "Privacy Policy", url: "https://openai.com/policies/privacy-policy/" },
    ],
  },
  "openrouter.ai": {
    org: "OpenRouter",
    links: [
      { label: "Terms", url: "https://openrouter.ai/terms" },
      { label: "Privacy Policy", url: "https://openrouter.ai/privacy" },
    ],
  },
  "generativelanguage.googleapis.com": {
    org: "Google",
    links: [
      { label: "Gemini API Terms", url: "https://ai.google.dev/gemini-api/terms" },
      { label: "Privacy Policy", url: "https://policies.google.com/privacy" },
    ],
  },
};

/** 送り先の URL の規約。知らない送り先なら `null` (利用者に規約を確かめるよう促す)。 */
export function termsFor(endpoint: string): ProviderTerms | null {
  let host: string;
  try {
    host = new URL(endpoint).hostname;
  } catch {
    return null;
  }
  return TERMS_BY_HOST[host] ?? null;
}
