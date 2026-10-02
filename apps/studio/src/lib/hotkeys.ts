// ホットキーのキーの書き方 (T0803、docs/studio.md#ホットキー)。純粋な関数だけを置く。
//
// キーは tauri-plugin-global-shortcut の書き方で持つ: 修飾キー (`Ctrl`・`Alt`・`Shift`・`Super`) と、
// `KeyboardEvent.code` の名前 (`KeyA`・`Digit1`・`F1`・`Numpad1`・`ArrowUp` など。配列に依らない物理的なキー) を `+` でつなぐ。
// 画面の表記は @ugoe/ui の Kbd (`"Ctrl+Shift+A"` の形) に直して出す。
import type { HotkeyAction, HotkeyKind } from "./inputs";

/** キーとして受け付ける `KeyboardEvent.code` (tauri-plugin-global-shortcut が読めるもの)。 */
const NAMED_CODES = new Set([
  "Backquote",
  "Backslash",
  "BracketLeft",
  "BracketRight",
  "Comma",
  "Equal",
  "Minus",
  "Period",
  "Quote",
  "Semicolon",
  "Slash",
  "Backspace",
  "Enter",
  "Space",
  "Tab",
  "Delete",
  "End",
  "Home",
  "Insert",
  "PageDown",
  "PageUp",
  "PrintScreen",
  "ScrollLock",
  "Pause",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "ArrowUp",
  "NumpadAdd",
  "NumpadDecimal",
  "NumpadDivide",
  "NumpadEnter",
  "NumpadEqual",
  "NumpadMultiply",
  "NumpadSubtract",
]);

/** 受け付けるキーか。 */
export function isSupportedCode(code: string): boolean {
  return (
    NAMED_CODES.has(code) ||
    /^Key[A-Z]$/.test(code) ||
    /^Digit[0-9]$/.test(code) ||
    /^Numpad[0-9]$/.test(code) ||
    /^F([1-9]|1[0-9]|2[0-4])$/.test(code)
  );
}

/** キーを押したときの修飾キーとキー (`KeyboardEvent` の必要なところだけ)。 */
export interface KeyPress {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

/**
 * 押したキーを割り当てのキーにする。修飾キーだけ・受け付けないキーなら `null` (押し続けて次のキーを待つ)。
 */
export function keysFromPress(press: KeyPress): string | null {
  if (!isSupportedCode(press.code)) return null;
  const parts: string[] = [];
  if (press.ctrlKey) parts.push("Ctrl");
  if (press.altKey) parts.push("Alt");
  if (press.shiftKey) parts.push("Shift");
  if (press.metaKey) parts.push("Super");
  parts.push(press.code);
  return parts.join("+");
}

/** 修飾キーが付いているか (付いていないキーはほかのアプリでの文字の入力を奪う)。 */
export function hasModifier(keys: string): boolean {
  return keys.split("+").length > 1;
}

/** 画面の表記 (@ugoe/ui の Kbd に渡す `"Ctrl+Shift+A"` の形)。 */
export function displayKeys(keys: string): string {
  return keys
    .split("+")
    .map((part) => {
      if (part === "Super") return "Meta";
      const letter = /^Key([A-Z])$/.exec(part);
      if (letter) return letter[1];
      const digit = /^Digit([0-9])$/.exec(part);
      if (digit) return digit[1];
      return part;
    })
    .join("+");
}

/** 種類を替えたときの操作の既定 (引数は選択肢の先頭)。 */
export function defaultAction(
  kind: HotkeyKind,
  options: { scenes: string[]; expressions: string[]; motions: string[]; slots: string[] },
): HotkeyAction {
  switch (kind) {
    case "scene":
      return { kind, scene: options.scenes[0] ?? "" };
    case "expression":
      return { kind, expression: options.expressions[0] ?? "", mode: "set" };
    case "motion":
      return { kind, motion: options.motions[0] ?? "", priority: "normal" };
    case "dressUp":
      return { kind, slot: options.slots[0] ?? "", module: null };
    default:
      return { kind } as HotkeyAction;
  }
}

/** 割り当ての画面で選べる操作の種類 (並べる順)。 */
export const HOTKEY_KINDS: readonly HotkeyKind[] = [
  "scene",
  "expression",
  "clearExpressions",
  "motion",
  "stopMotions",
  "dressUp",
  "trackingPause",
  "trackingCalibrate",
  "lipSyncPause",
];
