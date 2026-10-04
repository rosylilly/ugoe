// 外部の AI サービスに送る前の同意の流れ (docs/editor.md#ai-の設定と送信の同意)。
// 送る IPC コマンドは、同意がなければ送らずに `needs_consent` を返す。`withConsent` はそのときに
// 確認のダイアログ (ConsentDialog.svelte) を出し、同意を付けて呼び直す。
import { formatNumber } from "@ugoe/i18n";
import { t } from "../i18n";
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
      throw new Error(t("consent.changed"));
    }
  }
  return { value: reply.value };
}

/** バイト数を読みやすく (1 KB = 1024 B)。数値は今の言語で書き、単位の記号は訳さない。 */
export function formatBytes(bytes: number): string {
  // i18n-ignore: 単位の記号 (言語に依らない)
  const units = ["KB", "MB", "GB"];
  if (bytes < 1024) return `${formatNumber(bytes, { useGrouping: false })} B`;
  let value = bytes / 1024;
  let unit = 0;
  // 丸めて 1024 に届くなら次の単位にする (1023.9 KB は 1024 KB でなく 1.0 MB)
  while (Math.round(value) >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = value < 10 ? 1 : 0;
  return `${formatNumber(value, { minimumFractionDigits: digits, maximumFractionDigits: digits, useGrouping: false })} ${units[unit]}`;
}

/** 送り先の規約へのリンク。 */
export interface ProviderTerms {
  /** データを扱う組織の名前。 */
  org: string;
  /** `label` は読むたびに今の言語で作る。 */
  links: readonly { readonly label: string; url: string }[];
}

/** 規約のリンク (名前は訳す。URL は訳さない)。 */
function link(key: TermKey, url: string): { readonly label: string; url: string } {
  return {
    get label() {
      return termLabel(key);
    },
    url,
  };
}

type TermKey = "commercialTerms" | "privacyPolicy" | "servicesAgreement" | "terms" | "geminiTerms";

function termLabel(key: TermKey): string {
  switch (key) {
    case "commercialTerms":
      return t("consent.terms.commercialTerms");
    case "privacyPolicy":
      return t("consent.terms.privacyPolicy");
    case "servicesAgreement":
      return t("consent.terms.servicesAgreement");
    case "terms":
      return t("consent.terms.terms");
    case "geminiTerms":
      return t("consent.terms.geminiTerms");
  }
}

/** 送り先のホストごとの規約 (確認の画面に示す)。 */
const TERMS_BY_HOST: Record<string, ProviderTerms> = {
  "api.anthropic.com": {
    org: "Anthropic",
    links: [
      link("commercialTerms", "https://www.anthropic.com/legal/commercial-terms"),
      link("privacyPolicy", "https://www.anthropic.com/legal/privacy"),
    ],
  },
  "api.openai.com": {
    org: "OpenAI",
    links: [
      link("servicesAgreement", "https://openai.com/policies/services-agreement/"),
      link("privacyPolicy", "https://openai.com/policies/privacy-policy/"),
    ],
  },
  "openrouter.ai": {
    org: "OpenRouter",
    links: [
      link("terms", "https://openrouter.ai/terms"),
      link("privacyPolicy", "https://openrouter.ai/privacy"),
    ],
  },
  "generativelanguage.googleapis.com": {
    org: "Google",
    links: [
      link("geminiTerms", "https://ai.google.dev/gemini-api/terms"),
      link("privacyPolicy", "https://policies.google.com/privacy"),
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
