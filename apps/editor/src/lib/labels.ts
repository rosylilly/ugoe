// 要素の選択欄と見出しに出す表記 (表示名と ID)。PSD の日本語のレイヤー名は ID が `Layer8` のようになるので、
// ID だけでは見分けられない (T0627)。DOM に依存しない純粋な関数だけを置く (labels.test.ts で検査する)。

/** ID と、あれば表示名を持つ要素 (メッシュ・デフォーマ・パーツ・パラメータ)。 */
export interface Named {
  id: string;
  name?: string | null;
}

/** 表示名と ID の表記 (`顔 (Layer8)`)。名前が空か ID と同じなら ID だけ。 */
export function elementLabel(name: string | null | undefined, id: string): string {
  const trimmed = name?.trim();
  return trimmed && trimmed !== id ? `${trimmed} (${id})` : id;
}

/** `items` の中の `id` の要素の表記。見つからなければ ID だけ。 */
export function labelOf(items: readonly object[] | undefined, id: string): string {
  const item = items?.find((i) => (i as Named).id === id) as Named | undefined;
  return elementLabel(item?.name, id);
}

/** 表示名だけ (空なら ID)。 */
export function nameOf(items: readonly object[] | undefined, id: string): string {
  const item = items?.find((i) => (i as Named).id === id) as Named | undefined;
  return item?.name?.trim() || id;
}

/** 選択欄の選択肢 (値は ID、表示は名前と ID)。並びは `items` の順。 */
export function elementOptions(
  items: readonly object[],
  exclude: readonly string[] = [],
): { id: string; label: string }[] {
  return (items as Named[])
    .filter((i) => !exclude.includes(i.id))
    .map((i) => ({ id: i.id, label: elementLabel(i.name, i.id) }));
}

/** 同じ名前を省く上限 (テクスチャの表記で、使っているメッシュを何個まで並べるか)。 */
const MAX_USERS = 2;

/** テクスチャの表記: 名前と、それを使うメッシュの表示名 (`Layer8.png — 顔`)。多ければ残りの数。 */
export function textureLabel(name: string, users: readonly string[]): string {
  const shown = users.filter((u) => u !== name).slice(0, MAX_USERS);
  if (shown.length === 0) return name;
  const rest = users.length - shown.length;
  return `${name} — ${shown.join(", ")}${rest > 0 ? ` +${rest}` : ""}`;
}
