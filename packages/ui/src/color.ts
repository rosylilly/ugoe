// ColorPicker が使う色の扱い (純粋な関数)。色は `#rrggbb` (小文字) の文字列で受け渡す。

/** 0〜255 の RGB。 */
export interface Rgb {
  r: number;
  g: number;
  b: number;
}

/** `#rgb`・`#rrggbb` (`#` は省いてよい) を `#rrggbb` (小文字) にする。読めなければ `null`。 */
export function parseHexColor(text: string): string | null {
  const s = text.trim().replace(/^#/, "").toLowerCase();
  if (/^[0-9a-f]{3}$/.test(s)) {
    return `#${[...s].map((c) => c + c).join("")}`;
  }
  if (/^[0-9a-f]{6}$/.test(s)) return `#${s}`;
  return null;
}

/** `#rrggbb` を RGB にする。読めなければ黒。 */
export function hexToRgb(hex: string): Rgb {
  const s = parseHexColor(hex) ?? "#000000";
  return {
    r: Number.parseInt(s.slice(1, 3), 16),
    g: Number.parseInt(s.slice(3, 5), 16),
    b: Number.parseInt(s.slice(5, 7), 16),
  };
}

/** RGB を `#rrggbb` にする (0〜255 に収め、整数に丸める)。 */
export function rgbToHex({ r, g, b }: Rgb): string {
  const part = (v: number) =>
    Math.round(Math.min(255, Math.max(0, Number.isFinite(v) ? v : 0)))
      .toString(16)
      .padStart(2, "0");
  return `#${part(r)}${part(g)}${part(b)}`;
}
