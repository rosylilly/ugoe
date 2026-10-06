// ビューポートのツールが window で受けるキー操作 (Esc・Delete・Ctrl (Cmd) + A・矢印など) の判定 (keys.test.ts)。
import { isInDialog, isTextEntry } from "../../shortcuts";

/**
 * ツールがキー操作を受けてよいか。入力欄 (その欄自身の操作を優先する) とダイアログの中 (後ろのビューポートの
 * 選択や頂点を見えないところで変えない) では受けない。
 */
export function acceptsToolKey(target: EventTarget | null): boolean {
  return !isTextEntry(target) && !isInDialog(target);
}

/**
 * 矢印キーで点を動かしてよいか。フォーカスがどこにも無い (ビューポートを押した後の body) か、ビューポートの中
 * (道具の並び・説明の帯のボタン。`[data-viewport]`) のときだけ受ける。ツリー・スライダー・タイムラインなど、
 * 矢印を自分で使う部品のキーは奪わない。
 */
export function acceptsNudgeKey(target: EventTarget | null): boolean {
  if (!acceptsToolKey(target)) return false;
  const el = target as { tagName?: string; closest?: (selector: string) => unknown } | null;
  if (el === null || el.tagName === "BODY" || el.tagName === "HTML") return true;
  return typeof el.closest === "function" && !!el.closest("[data-viewport]");
}

/** 矢印キー 1 回で動かす量 (CSS ピクセル)。Shift を押していれば `NUDGE_LARGE`。 */
export const NUDGE = 1;
export const NUDGE_LARGE = 10;

/**
 * 矢印キーで動かす量 (画面の CSS ピクセル。下が +y)。矢印でない・修飾キー (Ctrl・Cmd・Alt) 付きなら null。
 * ビューポートで点をドラッグする操作の、キーボードでの代わり (docs/editor.md#ビューポートの操作と代わりの手段)。
 */
export function nudgeOf(e: {
  key: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
}): [number, number] | null {
  if (e.ctrlKey || e.metaKey || e.altKey) return null;
  const step = e.shiftKey ? NUDGE_LARGE : NUDGE;
  switch (e.key) {
    case "ArrowLeft":
      return [-step, 0];
    case "ArrowRight":
      return [step, 0];
    case "ArrowUp":
      return [0, -step];
    case "ArrowDown":
      return [0, step];
    default:
      return null;
  }
}
