// NumberField と Slider が使う数値の扱い (純粋な関数)。
// 編集のための数値なので、桁の区切りを付けず、小数点は "." に固定する (docs/ui.md#数値日付単位)。

/** 値の範囲と刻み。 */
export interface NumberRange {
  min?: number;
  max?: number;
  step?: number;
}

/** `value` を `[min, max]` に収める。 */
export function clamp(value: number, min = -Infinity, max = Infinity): number {
  return Math.min(max, Math.max(min, value));
}

/** 刻み `step` の小数の桁数 (0.05 → 2、1 → 0、1e-7 → 7)。 */
export function decimalsOf(step: number): number {
  if (!Number.isFinite(step) || step <= 0) return 0;
  const [mantissa = "", exponent] = step.toString().split("e");
  const fraction = mantissa.split(".")[1]?.length ?? 0;
  return Math.max(0, fraction - Number(exponent ?? 0));
}

/** 桁数を toFixed に渡せる 0〜15 の整数にする。 */
function digitsOf(decimals: number): number {
  return Math.min(Math.max(0, Math.round(decimals)), 15);
}

/** 浮動小数点の誤差 (0.1 + 0.2) を、`decimals` 桁で丸めて消す。 */
export function roundTo(value: number, decimals: number): number {
  return Number(value.toFixed(digitsOf(decimals)));
}

/** 編集の欄に出す文字列。末尾の 0 は付けない (0.50 → "0.5")。 */
export function formatEditable(value: number, decimals: number): string {
  if (!Number.isFinite(value)) return "";
  const rounded = roundTo(value, decimals);
  // -0 を "0" にする
  const s = (Object.is(rounded, -0) ? 0 : rounded).toFixed(digitsOf(decimals));
  // 1e21 以上は toFixed が指数の表記 ("1.2e+30") を返すので、末尾を削らない
  return s.includes(".") && !s.includes("e") ? s.replace(/\.?0+$/, "") : s;
}

/**
 * 欄に入れた文字列を数値にする。読めなければ `null`。
 * 全角の数字と記号・小数点の "," (欧州の書き方) も受ける。空白は無視する。
 */
export function parseEditable(text: string): number | null {
  const normalized = text
    .replace(/[０-９．，－＋]/g, (c) => String.fromCharCode(c.charCodeAt(0) - 0xfee0))
    .replace(/[\s_]/g, "")
    .replace(/[−–]/g, "-")
    .replace(",", ".");
  if (!/^[-+]?(\d+\.?\d*|\.\d+)(e[-+]?\d+)?$/i.test(normalized)) return null;
  const value = Number(normalized);
  return Number.isFinite(value) ? value : null;
}

/**
 * `value` を `steps` 刻み (負なら減らす) だけ動かし、範囲に収めて刻みの桁で丸める。
 */
export function stepValue(value: number, steps: number, range: NumberRange): number {
  const step = range.step ?? 1;
  const next = clamp(value + step * steps, range.min, range.max);
  return roundTo(next, decimalsOf(step));
}
