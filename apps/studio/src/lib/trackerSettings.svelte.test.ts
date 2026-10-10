// @vitest-environment happy-dom
// $effect を動かすため、モジュールをブラウザ向けに変換する環境で走らせる。
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import { DEFAULT_INPUT, type InputSettings } from "./inputs";
import { rememberTrackerSettings, type TrackerView } from "./trackerSettings.svelte";

/** 偽のトラッカー (`running` も書き換えられる)。 */
type FakeTracker = { -readonly [K in keyof TrackerView]: TrackerView[K] };

function setup(initial: InputSettings) {
  const tracker = $state<FakeTracker>({
    smoothing: initial.smoothing,
    swapSides: initial.swapSides,
    trackBody: initial.trackBody,
    stopEyeBlink: initial.stopEyeBlink,
    tuning: initial.tuning,
    running: false,
    source: null,
    cameraId: "",
  });
  const state = $state({ settings: initial });
  const sent: InputSettings[] = [];
  const stop = $effect.root(() => {
    rememberTrackerSettings(
      () => tracker,
      () => state.settings,
      (next) => {
        sent.push(next);
        // バックエンドが受け取り、プロファイルの通知で返す。
        state.settings = next;
      },
    );
  });
  flushSync();
  return { tracker, state, sent, stop };
}

describe("rememberTrackerSettings", () => {
  it("remembers the adjustments and the camera that started", () => {
    const { tracker, sent, stop } = setup(DEFAULT_INPUT);
    expect(sent).toEqual([]);
    tracker.smoothing = 0.8;
    flushSync();
    expect(sent.at(-1)?.smoothing).toBe(0.8);
    tracker.running = true;
    tracker.source = { kind: "camera", label: "Cam X" };
    tracker.cameraId = "x";
    flushSync();
    expect(sent.at(-1)?.camera).toEqual({ id: "x", label: "Cam X" });
    expect(sent).toHaveLength(2);
    stop();
  });

  it("does not write the running camera into a profile opened later", () => {
    const { tracker, state, sent, stop } = setup(DEFAULT_INPUT);
    tracker.running = true;
    tracker.source = { kind: "camera", label: "Cam X" };
    tracker.cameraId = "x";
    flushSync();
    expect(sent).toHaveLength(1);
    // 別のプロファイルを開いた (カメラは Y。動いているカメラは替えない)。
    state.settings = { ...DEFAULT_INPUT, smoothing: 0.2, camera: { id: "y", label: "Cam Y" } };
    // 操作ウィンドウが新しいプロファイルの調整をトラッカーに当てる。
    tracker.smoothing = 0.2;
    flushSync();
    expect(sent).toHaveLength(1);
    expect(state.settings.camera).toEqual({ id: "y", label: "Cam Y" });
    stop();
  });

  it("remembers the tuning without sending the same values back", () => {
    const { tracker, state, sent, stop } = setup(DEFAULT_INPUT);
    tracker.tuning = { ...DEFAULT_INPUT.tuning, yaw: { gain: 2, deadZone: 3 } };
    flushSync();
    expect(sent).toHaveLength(1);
    expect(sent[0]?.tuning.yaw).toEqual({ gain: 2, deadZone: 3 });
    // バックエンドから、項目の並びの違う同じ調整が届いても送り返さない。
    const { yaw, pitch, roll, eyeBlink, jawOpen } = sent[0]?.tuning ?? DEFAULT_INPUT.tuning;
    state.settings = {
      ...state.settings,
      tuning: {
        jawOpen: { high: jawOpen.high, low: jawOpen.low },
        eyeBlink,
        roll,
        pitch,
        yaw: { deadZone: yaw.deadZone, gain: yaw.gain },
      },
    };
    tracker.tuning = { ...tracker.tuning };
    flushSync();
    expect(sent).toHaveLength(1);
    stop();
  });
});
