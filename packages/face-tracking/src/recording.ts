// 記録した値の列 (JSON) の形式。カメラの無い環境 (CI・ヘッドレス) での確認や、不具合の再現に使う。
//
// 記録するのは、頭の回転をプロファイルの向きの約束に直し、(選んでいれば) 左右を入れ替えた後、
// キャリブレーションと平滑化の前の値。
// 再生するとキャリブレーションと平滑化を通してから変換する。
//
// {
//   "format": "ugoe-face-tracking",
//   "version": 2,
//   "frames": [
//     { "t": 0, "head": { "yaw": 1.2, "pitch": -3.4, "roll": 0.5 }, "blendShapes": { "eyeBlinkLeft": 0.1, ... },
//       "body": { "yaw": 3.1, "roll": -1.0, "pitch": -12.5, "shoulderLeft": 0.1, "shoulderRight": 0.1 } }
//   ]
// }
//
// `t` は最初のフレームからの秒数 (昇順)。顔が見つからなかったフレームは記録しない (再生では直前の値を保つ)。
// `body` (版 2、T0645) は体のトラッキングの値で、体のトラッキングを使っていなければ省く。値の無いキー
// (映っていない部分) も省く。版 1 のファイル (`body` が無い) もそのまま読める。
import { BODY_KEYS, type BodyPose, NO_BODY, type TrackingFrame } from "./frame";

export const RECORDING_FORMAT = "ugoe-face-tracking";
export const RECORDING_VERSION = 2;

export interface RecordedFrame extends TrackingFrame {
  t: number;
}

export interface Recording {
  format: typeof RECORDING_FORMAT;
  version: number;
  frames: RecordedFrame[];
}

function isFiniteNumber(v: unknown): v is number {
  return typeof v === "number" && Number.isFinite(v);
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null;
}

/** JSON の文字列を読む。形式が違えば例外を投げる。 */
export function parseRecording(text: string): Recording {
  const data: unknown = JSON.parse(text);
  if (typeof data !== "object" || data === null) throw new Error("not a JSON object");
  const obj = data as Record<string, unknown>;
  if (obj.format !== RECORDING_FORMAT) throw new Error(`format is not "${RECORDING_FORMAT}"`);
  if (!isFiniteNumber(obj.version) || obj.version > RECORDING_VERSION) {
    throw new Error(`unsupported version: ${String(obj.version)}`);
  }
  if (!Array.isArray(obj.frames)) throw new Error("frames is not an array");
  const frames: RecordedFrame[] = [];
  let last = -Infinity;
  for (const [i, raw] of obj.frames.entries()) {
    const f: Record<string, unknown> = isObject(raw) ? raw : {};
    const head = f.head;
    if (
      !isFiniteNumber(f.t) ||
      f.t < last ||
      !isObject(head) ||
      !isFiniteNumber(head.yaw) ||
      !isFiniteNumber(head.pitch) ||
      !isFiniteNumber(head.roll) ||
      typeof f.blendShapes !== "object" ||
      f.blendShapes === null
    ) {
      throw new Error(`frames[${i}] is invalid`);
    }
    last = f.t;
    const blendShapes: Record<string, number> = {};
    for (const [name, value] of Object.entries(f.blendShapes)) {
      if (isFiniteNumber(value)) blendShapes[name] = value;
    }
    const frame: RecordedFrame = {
      t: f.t,
      head: { yaw: head.yaw, pitch: head.pitch, roll: head.roll },
      blendShapes,
    };
    if (typeof f.body === "object" && f.body !== null) {
      const raw = f.body as Record<string, unknown>;
      const body: BodyPose = { ...NO_BODY };
      for (const key of BODY_KEYS) {
        const v = raw[key];
        if (isFiniteNumber(v)) body[key] = v;
      }
      frame.body = body;
    }
    frames.push(frame);
  }
  return { format: RECORDING_FORMAT, version: RECORDING_VERSION, frames };
}

/** 記録を JSON にする (値は小数第 4 位まで)。 */
export function serializeRecording(frames: readonly RecordedFrame[]): string {
  const round = (v: number) => Math.round(v * 1e4) / 1e4;
  const out = {
    format: RECORDING_FORMAT,
    version: RECORDING_VERSION,
    frames: frames.map((f) => {
      const row: Record<string, unknown> = {
        t: round(f.t),
        head: { yaw: round(f.head.yaw), pitch: round(f.head.pitch), roll: round(f.head.roll) },
        blendShapes: Object.fromEntries(
          Object.entries(f.blendShapes).map(([name, value]) => [name, round(value)]),
        ),
      };
      if (f.body) {
        const body = f.body;
        // 値の無いキーは省く (JSON の null にしない)。
        row.body = Object.fromEntries(
          BODY_KEYS.flatMap((key) => {
            const v = body[key];
            return v === null ? [] : [[key, round(v)]];
          }),
        );
      }
      return row;
    }),
  };
  return `${JSON.stringify(out)}\n`;
}

/** 記録の長さ (秒)。 */
export function recordingDuration(recording: Recording): number {
  return recording.frames.at(-1)?.t ?? 0;
}

/** 時刻 `t` の値 (その時刻以前の最後のフレーム)。最初のフレームより前なら最初のフレーム。 */
export function frameAt(recording: Recording, t: number): TrackingFrame | null {
  const frames = recording.frames;
  if (frames.length === 0) return null;
  let lo = 0;
  let hi = frames.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if ((frames[mid] as RecordedFrame).t <= t) lo = mid;
    else hi = mid - 1;
  }
  return frames[lo] ?? null;
}
