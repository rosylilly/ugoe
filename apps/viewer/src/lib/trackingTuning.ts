// フェイストラッキングの調整 (感度と境目。@ugoe/face-tracking の tuning.ts) を localStorage に覚える (T0738)。
// 起動のときに読んで `tracker.setTuning` に当て、変わったら覚え直す (App.svelte)。
// 既定値 (「既定に戻す」を含む) はキーを消して忘れる。覚えた値は `sanitizeTuning` を通して読むので、壊れた値・範囲の外・
// 知らない項目は既定値になる (調整の項目が増えても、足りない項目は既定値)。
import {
  DEFAULT_TUNING,
  isDefaultTuning,
  sanitizeTuning,
  type TrackingTuning,
} from "@ugoe/face-tracking";
import { readStoredItem, removeStoredItem, writeStoredItem } from "@ugoe/i18n";
import { STORAGE_KEYS } from "./storageKeys";

/** 覚えた調整。覚えていない・読めない・JSON でなければ既定値。 */
export function loadTrackingTuning(storage?: Storage): TrackingTuning {
  const text = readStoredItem(STORAGE_KEYS["tracking.tuning"], storage);
  if (text === null) return DEFAULT_TUNING;
  try {
    return sanitizeTuning(JSON.parse(text));
  } catch {
    return DEFAULT_TUNING;
  }
}

/** 調整を覚える。既定値ならキーを消す。 */
export function saveTrackingTuning(tuning: TrackingTuning, storage?: Storage): void {
  const key = STORAGE_KEYS["tracking.tuning"];
  const value = sanitizeTuning(tuning);
  if (isDefaultTuning(value)) removeStoredItem(key, storage);
  else writeStoredItem(key, JSON.stringify(value), storage);
}
