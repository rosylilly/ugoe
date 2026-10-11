// 調整 (tuning.ts) を localStorage に覚える読み書き (T0738 のビューアから移した。T1678)。ビューアとエディタのプレビューで使う。
// キーはアプリが決めて渡す (ビューアは `ugoe.viewer.tracking.tuning`、エディタは `ugoe.editor.trackingTuning`)。
// アプリは起動のときに読んで `tracker.setTuning` に当て、変わったら覚え直す。
// 既定値 (「既定に戻す」を含む) はキーを消して忘れる。覚えた値は `sanitizeTuning` を通して読むので、壊れた値・範囲の外・
// 知らない項目は既定値になる (調整の項目が増えても、足りない項目は既定値)。
// 読み書きは @ugoe/i18n の `readStoredItem` などを通す (使えない localStorage でも例外を出さない)。
// tuning.ts は型のほかに何も import しない約束なので、別のファイルに置く。
import { readStoredItem, removeStoredItem, writeStoredItem } from "@ugoe/i18n";
import { DEFAULT_TUNING, isDefaultTuning, sanitizeTuning, type TrackingTuning } from "./tuning";

/** `key` に覚えた調整。覚えていない・読めない・JSON でなければ既定値。`storage` を省くと localStorage。 */
export function loadStoredTuning(key: string, storage?: Storage): TrackingTuning {
  const text = readStoredItem(key, storage);
  if (text === null) return DEFAULT_TUNING;
  try {
    return sanitizeTuning(JSON.parse(text));
  } catch {
    return DEFAULT_TUNING;
  }
}

/** 調整を `key` に覚える。既定値ならキーを消す。`storage` を省くと localStorage。 */
export function saveStoredTuning(key: string, tuning: TrackingTuning, storage?: Storage): void {
  const value = sanitizeTuning(tuning);
  if (isDefaultTuning(value)) removeStoredItem(key, storage);
  else writeStoredItem(key, JSON.stringify(value), storage);
}
