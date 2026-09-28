// 背景色。`<input type="color">` の `#rrggbb` と、WebPuppet の各成分 0..1 の変換。

/** `#rrggbb` (または `#rgb`) を 0..1 の `[r, g, b]` にする。読めなければ `null`。 */
export function parseHexColor(hex: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(hex.trim());
  if (!m?.[1]) return null;
  const digits = m[1].length === 3 ? [...m[1]].map((c) => c + c).join("") : m[1];
  return [0, 2, 4].map((i) => Number.parseInt(digits.slice(i, i + 2), 16) / 255) as [
    number,
    number,
    number,
  ];
}
