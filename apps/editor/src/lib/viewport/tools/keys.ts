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

/** Alt + 矢印キー 1 回でビューを動かす量 (CSS ピクセル)。Shift も押していれば `PAN_LARGE`。 */
export const PAN = 40;
export const PAN_LARGE = 200;

/**
 * Alt + 矢印キーでビューを動かす量 (`ViewportController.panBy` に渡す CSS ピクセル)。Alt + ドラッグのパンの
 * キーボードでの代わりなので、ドラッグと同じく絵を矢印の向きに動かす。Alt の無い矢印 (点を動かす) と、
 * Ctrl・Cmd 付きは null。
 */
export function panOf(e: {
  key: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
}): [number, number] | null {
  if (!e.altKey || e.ctrlKey || e.metaKey) return null;
  const dir = nudgeOf({ ...e, altKey: false, shiftKey: false });
  if (!dir) return null;
  const step = e.shiftKey ? PAN_LARGE : PAN;
  return [dir[0] * step, dir[1] * step];
}

/**
 * 点を 1 つずつ選ぶキー: `]` で次、`[` で前 (修飾キー無し)。それ以外は null。
 * Tab はフォーカスの移動に残す。
 */
export function pointStepOf(e: {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
}): 1 | -1 | null {
  if (e.ctrlKey || e.metaKey || e.altKey) return null;
  if (e.key === "]") return 1;
  if (e.key === "[") return -1;
  return null;
}

/**
 * `]` `[` で選ぶ次の点の番号 (`count` 個の点を番号順に巡る)。選んだ点が無ければ次は最初・前は最後、
 * あれば最後に選んだ点 (`picked` の末尾) の次・前。点が無ければ null。
 */
export function steppedPoint(picked: readonly number[], count: number, dir: 1 | -1): number | null {
  if (!(count > 0)) return null;
  const anchor = picked.at(-1);
  if (anchor === undefined || anchor < 0 || anchor >= count) return dir > 0 ? 0 : count - 1;
  return (anchor + dir + count) % count;
}

/** `]` `[` で選んだ点を見える所へ送るとき、ビューの端から空ける幅 (CSS ピクセル)。 */
export const REVEAL_MARGIN = 48;

/**
 * `]` `[` で選んだ点 (`point`、CSS ピクセル) を見える所へ送るためにビューを動かす量 (`ViewportController.panBy` に
 * 渡す CSS ピクセル)。点がビュー (`size` = 幅・高さ) の端から `margin` より内側にあれば動かさず (null)、外にあれば
 * 端から `margin` の所まで、軸ごとに足りない分だけ動かす。ビューが `2 * margin` より狭い軸は真ん中へ送る。
 * 拡大率は変えない。数でない値や大きさが 0 のビューでは null。
 */
export function revealPanOf(
  point: readonly [number, number],
  size: readonly [number, number],
  margin = REVEAL_MARGIN,
): [number, number] | null {
  const dx = revealAxis(point[0], size[0], margin);
  const dy = revealAxis(point[1], size[1], margin);
  if (dx === null || dy === null || (dx === 0 && dy === 0)) return null;
  return [dx, dy];
}

function revealAxis(p: number, length: number, margin: number): number | null {
  if (!Number.isFinite(p) || !(length > 0 && Number.isFinite(length))) return null;
  const m = Math.min(Math.max(margin, 0), length / 2);
  if (p < m) return m - p;
  if (p > length - m) return length - m - p;
  return 0;
}
