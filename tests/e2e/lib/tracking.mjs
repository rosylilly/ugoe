// トラッキングの入力の偽物 (tests/e2e/README.md#偽のカメラ)。
//
// 本物のカメラと MediaPipe の代わりに、決まった動きの記録 (@ugoe/face-tracking の記録の形) を再生する。
// 重い検出を模すときは、トラッカーが刻みごとに値を書くところ (`replayTime`) で、決めた時間だけ主スレッドを塞ぐ。
// T0812 の不具合 (検出が刻みより重いと刻みが溜まり、invoke の応答が何秒も遅れる) は、この重さで再現する。

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
    return t ? { status: t.status, error: t.error, fps: t.fps, source: t.source?.kind ?? null } : null;
  `);
}
