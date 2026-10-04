// トラッキングの入力: 偽のカメラ (tests/e2e/README.md#偽のカメラ) と、画像からのトラッキング。
//
// 本物のカメラと MediaPipe の代わりに、決まった動きの記録 (@ugoe/face-tracking の記録の形) を再生する。
// 重い検出を模すときは、トラッカーが刻みごとに値を書くところ (`replayTime`) で、決めた時間だけ主スレッドを塞ぐ。
// T0812 の不具合 (検出が刻みより重いと刻みが溜まり、invoke の応答が何秒も遅れる) は、この重さで再現する。
//
// 本物の MediaPipe を通す手順は、カメラの代わりに顔の画像をトラッキングのパネルの「File…」に入れる (`checkImageTracking`。
// tests/e2e/README.md#画像からのトラッキング)。

import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { out, shot } from "./harness.mjs";
import { waitFor } from "./webdriver.mjs";

/**
 * 画像からのトラッキングに使う顔: 見本のアバター (`assets/samples/avatar-params.jpg` の「Open=1 Form=-1」の升) の口を開けた顔。
 * MediaPipe が顔を見つけ、`jawOpen` が 0.5 ほどになる (作り方は tests/e2e/README.md#画像からのトラッキング)。
 */
export const FACE_IMAGE = join(import.meta.dirname, "..", "fixtures", "avatar-mouth-open.jpg");

/**
 * 頭を左右に振り (yaw ±20 度、4 秒)、傾け (roll ±15 度、3 秒)、口を開け閉めする記録 (JSON のテキスト)。
 * @param {number} seconds 長さ
 */
export function swingRecording(seconds = 6) {
  const frames = [];
  for (let i = 0; i <= seconds * 30; i++) {
    const t = i / 30;
    frames.push({
      t,
      head: {
        yaw: 20 * Math.sin((2 * Math.PI * t) / 4),
        pitch: 0,
        roll: 15 * Math.sin((2 * Math.PI * t) / 3),
      },
      blendShapes: {
        jawOpen: 0.5 + 0.5 * Math.sin(2 * Math.PI * t),
        eyeBlinkLeft: 0,
        eyeBlinkRight: 0,
      },
    });
  }
  return JSON.stringify({ format: "ugoe-face-tracking", version: 2, frames });
}

/**
 * ページの `window.__UGOE_TRACKER__` で記録の再生を始める。`costMs` が正なら、刻みごとの処理を
 * その時間だけ重くする (MediaPipe の検出 1 回の重さを模す)。
 */
export async function startFakeTracking(session, { costMs = 0, seconds = 6 } = {}) {
  await session.run(
    async (text, costMs) => {
      const tracker = window.__UGOE_TRACKER__;
      if (!tracker) throw new Error("window.__UGOE_TRACKER__ is missing");
      if (costMs > 0) {
        // `replayTime` は Svelte の $state のクラスのフィールドで、プロトタイプのアクセサになっている。
        // このインスタンスだけ、書くたびに costMs だけ塞ぐアクセサに差し替える。
        let proto = Object.getPrototypeOf(tracker);
        let descriptor = null;
        while (proto && !descriptor) {
          descriptor = Object.getOwnPropertyDescriptor(proto, "replayTime");
          proto = Object.getPrototypeOf(proto);
        }
        if (!descriptor?.set) throw new Error("FaceTracker.replayTime is not an accessor");
        Object.defineProperty(tracker, "replayTime", {
          configurable: true,
          get() {
            return descriptor.get.call(this);
          },
          set(value) {
            const end = performance.now() + costMs;
            while (performance.now() < end) {
              // 検出の重さ
            }
            descriptor.set.call(this, value);
          },
        });
      }
      await tracker.startReplay(text, "e2e");
      return tracker.status;
    },
    swingRecording(seconds),
    costMs,
  );
}

/** トラッカーの状態 (`status`・`fps`・`source`)。 */
export function trackerState(session) {
  return session.execute(`
    const t = window.__UGOE_TRACKER__;
    return t
      ? { status: t.status, error: t.error, fps: t.fps, source: t.source?.kind ?? null, faceFound: t.faceFound }
      : null;
  `);
}

/**
 * 今のトラッキングを止めてから画像でトラッキングを始め、本物の MediaPipe (同梱の wasm とモデル) が読み込まれて顔が見つかり、モデルのパラメータ
 * (既定は口の開き `MouthOpenY`。パネルのスライダーで読む) が動くことを確かめる。動いた画面を `name` のスクリーンショットに、
 * 値を `E2E_OUT/<name>.json` に残し、トラッキングを止める。
 *
 * 画像は WebDriver の入力でトラッキングのパネル (@ugoe/face-tracking の TrackingPanel) のファイルの入力欄に渡す。
 * OS のファイルダイアログは開かない。3 つのアプリ (エディタのプレビュー・ビューア・スタジオの操作ウィンドウ) で同じパネルを使う。
 * 呼ぶ前に `installPageHelpers` を済ませ、パラメータのスライダーが見えているようにしておく。
 *
 * @param {import("./webdriver.mjs").Session} session
 * @param {string} name 結果の名前 (スクリーンショットと JSON)
 * @param {{ image?: string, parameter?: string, timeout?: number }} options
 *   `parameter` はスライダーのラベルに合わせる正規表現。`timeout` は MediaPipe を読んで顔を見つけるまでの待ち。
 * @returns {Promise<{ before: number, after: number, tracker: object }>} `tracker` は値が動いた時の `trackerState`
 */
export async function checkImageTracking(
  session,
  name,
  { image = FACE_IMAGE, parameter = "^Mouth ?Open ?Y( |$)", timeout = 60_000 } = {},
) {
  const read = () => session.execute("return __e2e.sliderValue(arguments[0])", parameter);
  // 前のトラッキング (記録の再生など) を止め、パラメータが既定値に戻って落ち着いた値から比べる
  await session.execute("window.__UGOE_TRACKER__.stop()");
  const before = await waitFor(
    async () => {
      const first = await read();
      await new Promise((r) => setTimeout(r, 300));
      const value = await read();
      return value !== null && value === first ? { value } : null;
    },
    { message: `the slider ${parameter} to settle` },
  );
  // パネルは畳める (エディタのプレビューでは畳んで始まる)。畳んでいると中身が無いので開く
  const trigger = await session.waitFind(".tracking .ugoe-section__trigger");
  if ((await trigger.attribute("aria-expanded")) !== "true") await trigger.click();
  // 隠れたファイルの入力欄も、WebDriver の Element Send Keys でパスを受け取る (W3C WebDriver の file の入力)
  const input = await session.waitFind(".tracking input[type=file]");
  await input.type(image);
  const started = await waitFor(
    async () => {
      const state = await trackerState(session);
      if (state?.status === "error") return { failed: state };
      return state?.status === "running" && state.source === "image" && state.faceFound
        ? state
        : null;
    },
    { timeout, message: "MediaPipe to load and find the face in the image" },
  );
  if (started.failed) throw new Error(`the tracking failed: ${JSON.stringify(started.failed)}`);
  // 値は平滑化を通って少しずつ近づく
  const after = await waitFor(
    async () => {
      const value = await read();
      return value !== null && Math.abs(value - before.value) > 0.15 ? { value } : null;
    },
    { message: `${parameter} to follow the face in the image (from ${before.value})` },
  );
  const result = { before: before.value, after: after.value, tracker: await trackerState(session) };
  writeFileSync(join(out(), `${name}.json`), `${JSON.stringify(result, null, 2)}\n`);
  await shot(session, name);
  await session.execute("window.__UGOE_TRACKER__.stop()");
  return result;
}
