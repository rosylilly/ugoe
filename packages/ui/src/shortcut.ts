// ショートカットの表記 (Kbd・Menu・Tooltip が使う。純粋な関数)。docs/ui.md#部品の一覧 の Kbd。
// ショートカットは `"Mod+Shift+Z"` のように `+` でつないで書く。`Mod` は macOS では ⌘、ほかでは Ctrl。

/** 表記を決める OS の区分。 */
export type Platform = "mac" | "other";

/** 今の環境の区分 (macOS・iOS なら `mac`)。 */
export function currentPlatform(): Platform {
  if (typeof navigator === "undefined") return "other";
  const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
  const name = nav.userAgentData?.platform || nav.platform || "";
  return /mac|iphone|ipad/i.test(name) ? "mac" : "other";
}

type Modifier = "ctrl" | "alt" | "shift" | "meta";

/** 修飾キーの並び (macOS は ⌃⌥⇧⌘、ほかは Ctrl+Alt+Shift+Meta の順に並べる)。 */
const MODIFIER_ORDER: Modifier[] = ["ctrl", "alt", "shift", "meta"];

const MAC_MODIFIER: Record<Modifier, string> = { ctrl: "⌃", alt: "⌥", shift: "⇧", meta: "⌘" };
const OTHER_MODIFIER: Record<Modifier, string> = {
  ctrl: "Ctrl",
  // i18n-ignore: キーボードに刻まれたキーの名前は訳さない
  alt: "Alt",
  shift: "Shift",
  meta: "Win",
};
const ARIA_MODIFIER: Record<Modifier, string> = {
  ctrl: "Control",
  // i18n-ignore: aria-keyshortcuts の値 (キーの名前) は訳さない
  alt: "Alt",
  shift: "Shift",
  meta: "Meta",
};

/** キーの名前の表記 (どの OS でも同じもの)。キーの名前は KeyboardEvent.key の値で書く。 */
const KEY_LABEL: Record<string, string> = {
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Escape: "Esc",
  " ": "Space",
};

/** macOS で記号にするキー。 */
const MAC_KEY_LABEL: Record<string, string> = {
  Enter: "↩",
  Backspace: "⌫",
  Delete: "⌦",
  Tab: "⇥",
};

/** 解釈したショートカット。 */
export interface Shortcut {
  modifiers: Modifier[];
  /** 修飾キーでないキー (KeyboardEvent.key の値。1 文字は大文字にする)。 */
  key: string;
}

/** `"Mod+Shift+Z"` を解釈する。`Mod` は区分に応じて Meta (macOS) か Ctrl になる。 */
export function parseShortcut(text: string, platform: Platform): Shortcut {
  const parts = text.split("+").map((p) => p.trim());
  // "Ctrl++" のように "+" そのものをキーにしたとき
  if (text.endsWith("++")) parts.splice(-2, 2, "+");
  const modifiers = new Set<Modifier>();
  let key = "";
  for (const part of parts) {
    switch (part.toLowerCase()) {
      case "mod":
        modifiers.add(platform === "mac" ? "meta" : "ctrl");
        break;
      case "ctrl":
      case "control":
        modifiers.add("ctrl");
        break;
      case "alt":
      case "option":
        modifiers.add("alt");
        break;
      case "shift":
        modifiers.add("shift");
        break;
      case "meta":
      case "cmd":
      case "command":
        modifiers.add("meta");
        break;
      case "space":
        key = " ";
        break;
      case "esc":
        key = "Escape";
        break;
      default:
        key = part.length === 1 ? part.toUpperCase() : part;
    }
  }
  return { modifiers: MODIFIER_ORDER.filter((m) => modifiers.has(m)), key };
}

/** 画面に出すキーの並び (macOS は ⌘⇧Z、ほかは Ctrl・Shift・Z)。 */
export function shortcutKeys(text: string, platform: Platform = currentPlatform()): string[] {
  const { modifiers, key } = parseShortcut(text, platform);
  const names = platform === "mac" ? MAC_MODIFIER : OTHER_MODIFIER;
  const keyLabel = (platform === "mac" ? MAC_KEY_LABEL[key] : undefined) ?? KEY_LABEL[key] ?? key;
  return [...modifiers.map((m) => names[m]), ...(key ? [keyLabel] : [])];
}

/** 1 行の文字にした表記 (macOS は "⌘⇧Z"、ほかは "Ctrl+Shift+Z")。 */
export function formatShortcut(text: string, platform: Platform = currentPlatform()): string {
  return shortcutKeys(text, platform).join(platform === "mac" ? "" : "+");
}

/** `aria-keyshortcuts` の値 ("Meta+Shift+Z" / "Control+Shift+Z")。 */
export function ariaKeyShortcuts(text: string, platform: Platform = currentPlatform()): string {
  const { modifiers, key } = parseShortcut(text, platform);
  const ariaKey = key === " " ? "Space" : key;
  return [...modifiers.map((m) => ARIA_MODIFIER[m]), ...(key ? [ariaKey] : [])].join("+");
}
