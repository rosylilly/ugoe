// 部品の中で、Bits UI のトリガーの属性 (Tooltip・Menu・Popover) と ugoe の props を重ねる。

/** トリガーにする要素 (ボタンなど) に広げる属性。Tooltip・Menu・Popover の `trigger` のスニペットが受け取る。 */
export type TriggerAttributes = Record<string, unknown>;

/**
 * 属性を重ねる。`on` で始まる関数 (イベントの受け手) は両方を順に呼び、ほかは後のものが勝つ。
 * Svelte の attachment (symbol のキー) もそのまま写す。
 */
export function mergeAttributes(...all: (TriggerAttributes | undefined)[]): TriggerAttributes {
  const out: TriggerAttributes = {};
  for (const attrs of all) {
    if (!attrs) continue;
    for (const key of Reflect.ownKeys(attrs)) {
      const next = (attrs as Record<string | symbol, unknown>)[key];
      const prev = (out as Record<string | symbol, unknown>)[key];
      if (
        typeof key === "string" &&
        key.startsWith("on") &&
        typeof prev === "function" &&
        typeof next === "function"
      ) {
        (out as Record<string, unknown>)[key] = (...args: unknown[]) => {
          prev(...args);
          next(...args);
        };
      } else if (next !== undefined || !(key in out)) {
        (out as Record<string | symbol, unknown>)[key] = next;
      }
    }
  }
  return out;
}
