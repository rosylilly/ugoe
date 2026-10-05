// 右の列 (インスペクタとパラメータ) の上下の分け方 (docs/editor.md#画面構成-m2-の目標)。DOM に依存しない部分だけを置き、
// 覚えておく値は localStorage に入れる (使えない環境では既定値)。layout.test.ts で検査する。
import { readStoredItem, writeStoredItem } from "@ugoe/i18n";

/** インスペクタの高さの割合の既定値 (残りがパラメータ)。 */
export const DEFAULT_SPLIT = 0.6;
/** 片方が見えなくならないように、割合をこの範囲に収める。 */
export const MIN_SPLIT = 0.15;
export const MAX_SPLIT = 0.85;
/** 矢印キーで動かす量。 */
export const SPLIT_STEP = 0.05;

const KEY = "ugoe.editor.rightSplit";

/** 割合を範囲に収める (数でなければ既定値)。 */
export function clampSplit(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_SPLIT;
  return Math.min(MAX_SPLIT, Math.max(MIN_SPLIT, value));
}

/** 区切りの位置 (列の上端からの距離 `offset`、列の高さ `height`) の割合。 */
export function splitAt(offset: number, height: number): number {
  return clampSplit(height > 0 ? offset / height : DEFAULT_SPLIT);
}

/** 区切りのキー操作 (上下の矢印・Home・End) の後の割合。関係ないキーなら null。 */
export function splitByKey(split: number, key: string): number | null {
  switch (key) {
    case "ArrowUp":
      return clampSplit(split - SPLIT_STEP);
    case "ArrowDown":
      return clampSplit(split + SPLIT_STEP);
    case "Home":
      return MIN_SPLIT;
    case "End":
      return MAX_SPLIT;
    default:
      return null;
  }
}

export function loadSplit(): number {
  const saved = readStoredItem(KEY);
  return saved == null ? DEFAULT_SPLIT : clampSplit(Number(saved));
}

/** 覚えられなくても、この画面では今の値を使い続ける。 */
export function saveSplit(split: number): void {
  writeStoredItem(KEY, String(split));
}
