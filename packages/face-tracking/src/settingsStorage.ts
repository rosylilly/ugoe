// 調整のほかのトラッキングの設定 (平滑化・左右の入れ替え・目パチを止める・体) を localStorage に覚える読み書き (T0739)。
// 調整 (tuningStorage.ts) と同じ形: キーはアプリが決めて渡し (ビューアは `ugoe.viewer.tracking.settings`、エディタは `ugoe.editor.trackingSettings`)、1 つのキーに JSON で置く。
// アプリは起動のときに読んで `applyTrackerSettings` で当て、変わったら `trackerSettings` で取り出して覚え直す。
// 既定値はキーを消して忘れる。読むときは `sanitizeTrackingSettings` を通すので、壊れた項目・範囲の外・知らない項目は項目ごとに既定値になる。
// 覚えないもの: カメラ (機器の ID は機械や接続で変わる)・ランドマークと鏡像の表示 (見た目だけで、その場で切り替えるもの)。
import { readStoredItem, removeStoredItem, writeStoredItem } from "@ugoe/i18n";

/** 覚えるトラッキングの設定 (`FaceTracker` の同じ名前の値)。 */
export interface TrackingSettings {
  /** 平滑化の強さ (0〜1)。 */
  smoothing: number;
  /** 左右を入れ替える。 */
  swapSides: boolean;
  /** トラッキング中はモデルの自動の目パチを止める。 */
  stopEyeBlink: boolean;
  /** 体のトラッキングも行う。 */
  trackBody: boolean;
}

/** 既定値 (`FaceTracker` を作ったときの値。tracker.test.ts で一致を検査する)。 */
export const DEFAULT_TRACKING_SETTINGS: TrackingSettings = Object.freeze({
  smoothing: 0.5,
  swapSides: false,
  stopEyeBlink: true,
  trackBody: true,
});

/** 設定を持つもの (`FaceTracker`)。目パチと体は、動いている間の切り替えも行う関数を通して変える。 */
export interface TrackingSettingsHolder {
  smoothing: number;
  swapSides: boolean;
  readonly stopEyeBlink: boolean;
  readonly trackBody: boolean;
  setStopEyeBlink(stop: boolean): void;
  setTrackBody(on: boolean): void;
}

/** 外から来た値を設定にする。オブジェクトでなければ既定値。項目ごとに、型の違う値は既定値、平滑化は 0〜1 に収める。 */
export function sanitizeTrackingSettings(value: unknown): TrackingSettings {
  const v = typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
  const flag = (x: unknown, fallback: boolean) => (typeof x === "boolean" ? x : fallback);
  const smoothing =
    typeof v.smoothing === "number" && Number.isFinite(v.smoothing)
      ? Math.min(1, Math.max(0, v.smoothing))
      : DEFAULT_TRACKING_SETTINGS.smoothing;
  return {
    smoothing,
    swapSides: flag(v.swapSides, DEFAULT_TRACKING_SETTINGS.swapSides),
    stopEyeBlink: flag(v.stopEyeBlink, DEFAULT_TRACKING_SETTINGS.stopEyeBlink),
    trackBody: flag(v.trackBody, DEFAULT_TRACKING_SETTINGS.trackBody),
  };
}

/** 既定値と同じか。 */
export function isDefaultTrackingSettings(settings: TrackingSettings): boolean {
  return (Object.keys(DEFAULT_TRACKING_SETTINGS) as (keyof TrackingSettings)[]).every(
    (name) => settings[name] === DEFAULT_TRACKING_SETTINGS[name],
  );
}

/** `tracker` の今の設定。 */
export function trackerSettings(tracker: TrackingSettingsHolder): TrackingSettings {
  return {
    smoothing: tracker.smoothing,
    swapSides: tracker.swapSides,
    stopEyeBlink: tracker.stopEyeBlink,
    trackBody: tracker.trackBody,
  };
}

/** 設定を `tracker` に当てる (範囲に収めてから)。 */
export function applyTrackerSettings(
  tracker: TrackingSettingsHolder,
  settings: TrackingSettings,
): void {
  const value = sanitizeTrackingSettings(settings);
  tracker.smoothing = value.smoothing;
  tracker.swapSides = value.swapSides;
  if (tracker.stopEyeBlink !== value.stopEyeBlink) tracker.setStopEyeBlink(value.stopEyeBlink);
  if (tracker.trackBody !== value.trackBody) tracker.setTrackBody(value.trackBody);
}

/** `key` に覚えた設定。覚えていない・読めない・JSON でなければ既定値。`storage` を省くと localStorage。 */
export function loadStoredSettings(key: string, storage?: Storage): TrackingSettings {
  const text = readStoredItem(key, storage);
  if (text === null) return DEFAULT_TRACKING_SETTINGS;
  try {
    return sanitizeTrackingSettings(JSON.parse(text));
  } catch {
    return DEFAULT_TRACKING_SETTINGS;
  }
}

/** 設定を `key` に覚える。既定値ならキーを消す。`storage` を省くと localStorage。 */
export function saveStoredSettings(
  key: string,
  settings: TrackingSettings,
  storage?: Storage,
): void {
  const value = sanitizeTrackingSettings(settings);
  if (isDefaultTrackingSettings(value)) removeStoredItem(key, storage);
  else writeStoredItem(key, JSON.stringify(value), storage);
}
