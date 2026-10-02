// 訳文の中の印 (`<link>開く</link>`) を区切る (RichText.svelte)。文の一部だけを強調・リンクにしたいとき、
// 文を分けずに訳せるようにする (docs/ui.md#引数)。印の中に印は入れられない。HTML としては解釈しない。

export type RichPart = { kind: "text"; text: string } | { kind: "tag"; name: string; text: string };

const TAG = /<([A-Za-z][\w-]*)>(.*?)<\/\1>/gs;

/** 訳文を、ただの文と印 (`<name>中身</name>`) の並びに分ける。閉じていない印はただの文として残す。 */
export function parseRich(text: string): RichPart[] {
  const parts: RichPart[] = [];
  let last = 0;
  for (const m of text.matchAll(TAG)) {
    if (m.index > last) parts.push({ kind: "text", text: text.slice(last, m.index) });
    parts.push({ kind: "tag", name: m[1] ?? "", text: m[2] ?? "" });
    last = m.index + m[0].length;
  }
  if (last < text.length) parts.push({ kind: "text", text: text.slice(last) });
  return parts;
}
