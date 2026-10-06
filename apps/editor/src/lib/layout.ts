// 画面の分け方 (docs/editor.md#画面の配置)。左右の列の幅・タイムラインの高さ・右の列の上下の分け方を、
// 区切り (lib/ui/Splitter.svelte) のポインタとキーで変え、localStorage に覚える (使えない環境では既定値)。
// DOM に依存しない部分だけを置く。layout.test.ts で検査する。
import { readStoredItem, removeStoredItem, writeStoredItem } from "@ugoe/i18n";
import { STORAGE_KEYS } from "./storageKeys";

/** 区切りで変える大きさの範囲 (CSS px か割合) と、覚えるキー。 */
export interface SizeRange {
  /** localStorage のキー (storageKeys.ts)。 */
  key: string;
  min: number;
  max: number;
  /** 既定値。null なら CSS の既定 (タイムラインの高さ。窓の高さに合わせる)。 */
  default: number | null;
  /** 矢印キーで動かす量。 */
  step: number;
}

/** 左の列 (ツリー) の幅。 */
export const LEFT_WIDTH: SizeRange = {
  key: STORAGE_KEYS.leftWidth,
  min: 180,
  max: 420,
  default: 240,
  step: 16,
};

/** 右の列 (インスペクタとパラメータ) の幅。 */
export const RIGHT_WIDTH: SizeRange = {
  key: STORAGE_KEYS.rightWidth,
  min: 280,
  max: 560,
  default: 320,
  step: 16,
};

/** タイムライン (開いているとき) の高さ。既定は CSS の `clamp(280px, 42vh, 520px)`。 */
export const TIMELINE_HEIGHT: SizeRange = {
  key: STORAGE_KEYS.timelineHeight,
  min: 160,
  max: 800,
  default: null,
  step: 24,
};

/** 右の列の上下の分け方 (インスペクタの高さの割合。残りがパラメータ)。 */
export const RIGHT_SPLIT: SizeRange = {
  key: STORAGE_KEYS.rightSplit,
  min: 0.15,
  max: 0.85,
  default: 0.6,
  step: 0.05,
};

/** インスペクタの高さの割合の既定値 (残りがパラメータ)。 */
export const DEFAULT_SPLIT = 0.6;
/** 片方が見えなくならないように、割合をこの範囲に収める。 */
export const MIN_SPLIT = RIGHT_SPLIT.min;
export const MAX_SPLIT = RIGHT_SPLIT.max;
/** 矢印キーで動かす量。 */
export const SPLIT_STEP = RIGHT_SPLIT.step;

/** 大きさを範囲に収める (数でなければ `fallback`)。 */
export function clampSize(range: SizeRange, value: number, fallback: number): number {
  if (!Number.isFinite(value)) return fallback;
  return Math.min(range.max, Math.max(range.min, value));
}

/**
 * 区切りのキー操作の後の大きさ。関係ないキーなら null。
 * - `orientation` は区切りの線の向き: 列の間の縦の線 (`vertical`) は左右の矢印、上下の間の横の線 (`horizontal`) は上下の矢印で動かす。
 * - `reverse` は、区切りが領域の手前 (右の列の左端・タイムラインの上端) にあること。線を左 (上) へ動かすと領域が大きくなる。
 * - Home は最小、End は最大。
 */
export function sizeByKey(
  range: SizeRange,
  value: number,
  key: string,
  orientation: "vertical" | "horizontal",
  reverse = false,
): number | null {
  const [back, forward] =
    orientation === "vertical" ? ["ArrowLeft", "ArrowRight"] : ["ArrowUp", "ArrowDown"];
  let delta: number;
  if (key === forward) delta = range.step;
  else if (key === back) delta = -range.step;
  else if (key === "Home") return range.min;
  else if (key === "End") return range.max;
  else return null;
  return clampSize(range, value + (reverse ? -delta : delta), value);
}

/** 覚えた大きさ (無い・読めない・数でなければ既定値、範囲の外は範囲に収める)。 */
export function loadSize(range: SizeRange): number | null {
  const saved = readStoredItem(range.key);
  if (saved == null || saved.trim() === "") return range.default;
  const value = Number(saved);
  return Number.isFinite(value) ? clampSize(range, value, value) : range.default;
}

/** 覚えられなくても、この画面では今の値を使い続ける。null は既定に戻す (覚えた値を消す)。 */
export function saveSize(range: SizeRange, value: number | null): void {
  if (value === null) removeStoredItem(range.key);
  else writeStoredItem(range.key, String(value));
}

/** 割合を範囲に収める (数でなければ既定値)。 */
export function clampSplit(value: number): number {
  return clampSize(RIGHT_SPLIT, value, DEFAULT_SPLIT);
}

/** 区切りの位置 (列の上端からの距離 `offset`、列の高さ `height`) の割合。 */
export function splitAt(offset: number, height: number): number {
  return clampSplit(height > 0 ? offset / height : DEFAULT_SPLIT);
}

/** 区切りのキー操作 (上下の矢印・Home・End) の後の割合。関係ないキーなら null。 */
export function splitByKey(split: number, key: string): number | null {
  return sizeByKey(RIGHT_SPLIT, split, key, "horizontal");
}

export function loadSplit(): number {
  return loadSize(RIGHT_SPLIT) ?? DEFAULT_SPLIT;
}

/** 覚えられなくても、この画面では今の値を使い続ける。 */
export function saveSplit(split: number): void {
  saveSize(RIGHT_SPLIT, split);
}
