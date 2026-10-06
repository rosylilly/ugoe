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

/**
 * 2 つ目の点を選ぶキー: Shift + `]` で次、Shift + `[` で前 (メッシュ編集。Shift + `]` `[` の `key` は配列によって
 * `}` `{` になるので両方を受ける)。それ以外は null。
 */
export function pointExtendOf(e: {
  key: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
}): 1 | -1 | null {
  if (!e.shiftKey || e.ctrlKey || e.metaKey || e.altKey) return null;
  if (e.key === "}" || e.key === "]") return 1;
  if (e.key === "{" || e.key === "[") return -1;
  return null;
}

/**
 * Shift + `]` `[` で選ぶ 2 つの点の番号。1 つ目 (`picked` の先頭) を残し、2 つ目 (`picked` の末尾。1 つしか無ければ
 * 1 つ目) の次・前の点を 2 つ目にする (1 つ目は飛ばす)。選んだ点が無ければ `steppedPoint` と同じ 1 つ。
 * 点が 2 つ未満なら 1 つ目だけ、点が無ければ null。メッシュ編集で辺を分ける 2 つの頂点を選ぶ (`I`)。
 */
export function extendedPoints(
  picked: readonly number[],
  count: number,
  dir: 1 | -1,
): number[] | null {
  const valid = picked.filter((i) => i >= 0 && i < count);
  const first = valid[0];
  if (first === undefined) {
    const only = steppedPoint([], count, dir);
    return only === null ? null : [only];
  }
  if (count < 2) return [first];
  const last = valid.length >= 2 ? (valid.at(-1) ?? first) : first;
  let next = (last + dir + count) % count;
  if (next === first) next = (next + dir + count) % count;
  return [first, next];
}

/** 選んだ 2 つの頂点の中点に頂点を足すキー: `I` (修飾キー無し。メッシュ編集で辺を分ける)。 */
export function isSplitKey(e: {
  key: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
}): boolean {
  return !(e.shiftKey || e.ctrlKey || e.metaKey || e.altKey) && (e.key === "i" || e.key === "I");
}

/** `]` `[` で選んだ点を見える所へ送るとき、見える矩形の端から空ける幅 (CSS ピクセル)。 */
export const REVEAL_MARGIN = 48;

/** ビューポートで実際に見える矩形 (CSS ピクセル。ビューポート要素の左上原点)。 */
export interface ViewRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * ビューポート要素の大きさ `size` (幅・高さ) から、上の端の `covered.top`・下の端の `covered.bottom` の高さを重ねた
 * 道具が隠す分 (左上の道具の並びと説明の帯・右下の表示の道具) を除いた、見える矩形。隠す高さが数でない・負なら 0 とし、
 * 上と下を合わせてビューの高さ以上を隠すときは除かない (ビュー全体)。
 */
export function visibleRectOf(
  size: readonly [number, number],
  covered: { readonly top: number; readonly bottom: number },
): ViewRect {
  const [width, height] = size;
  const top = Number.isFinite(covered.top) ? Math.max(covered.top, 0) : 0;
  const bottom = Number.isFinite(covered.bottom) ? Math.max(covered.bottom, 0) : 0;
  if (!(top + bottom < height)) return { left: 0, top: 0, width, height };
  return { left: 0, top, width, height: height - top - bottom };
}

/**
 * `]` `[` で選んだ点 (`point`、CSS ピクセル) を見える所へ送るためにビューを動かす量 (`ViewportController.panBy` に
 * 渡す CSS ピクセル)。点が見える矩形 (`view`。visibleRectOf) の端から `margin` より内側にあれば動かさず (null)、外に
 * あれば端から `margin` の所まで、軸ごとに足りない分だけ動かす。矩形が `2 * margin` より狭い軸は真ん中へ送る。
 * 拡大率は変えない。数でない値や大きさが 0 の矩形では null。
 */
export function revealPanOf(
  point: readonly [number, number],
  view: ViewRect,
  margin = REVEAL_MARGIN,
): [number, number] | null {
  const dx = revealAxis(point[0], view.left, view.width, margin);
  const dy = revealAxis(point[1], view.top, view.height, margin);
  if (dx === null || dy === null || (dx === 0 && dy === 0)) return null;
  return [dx, dy];
}

function revealAxis(p: number, start: number, length: number, margin: number): number | null {
  if (!Number.isFinite(p) || !Number.isFinite(start)) return null;
  if (!(length > 0 && Number.isFinite(length))) return null;
  const m = Math.min(Math.max(margin, 0), length / 2);
  if (p < start + m) return start + m - p;
  if (p > start + length - m) return start + length - m - p;
  return 0;
}
