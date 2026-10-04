// @vitest-environment happy-dom
// 入力の設定 (InputPanel.svelte): プロファイルの通知を待たずに続けて変えても、変更が消えないこと。
import { FaceTracker } from "@ugoe/face-tracking";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import InputPanel from "./InputPanel.svelte";
import { DEFAULT_INPUT, type InputSettings } from "./lib/inputs";
import { Microphone } from "./lib/microphone.svelte";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

function setup(initial: InputSettings) {
  const sent: InputSettings[] = [];
  const finish: (() => void)[] = [];
  // カメラも MediaPipe も使わない (処理は回さない)。
  const tracker = new FaceTracker(
    { setTracking: () => {}, setAutoEyeBlink: () => {} },
    { schedule: () => () => {} },
  );
  const props = $state({
    tracker,
    microphone: new Microphone(
      () => {},
      (): InputSettings => props.settings,
    ),
    settings: initial,
    trackingParameters: [],
    values: {},
    trackingPaused: false,
    lipSyncPaused: false,
    onsettings: (settings: InputSettings) => {
      sent.push(settings);
      return new Promise<void>((resolve) => finish.push(resolve));
    },
    ontrackingpause: () => {},
    onlipsyncpause: () => {},
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(InputPanel, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  /** バックエンドが送った設定を受け取り、通知で返してから送り終える (送っていないものが無くなるまで)。 */
  const notify = async () => {
    for (let resolve = finish.shift(); resolve !== undefined; resolve = finish.shift()) {
      const last = sent.at(-1);
      if (last !== undefined) props.settings = last;
      resolve();
      await new Promise((r) => setTimeout(r, 0));
    }
    flushSync();
  };
  return { target, tracker, props, sent, notify };
}

describe("InputPanel", () => {
  it("keeps every adjustment made before the profile notification arrives", async () => {
    const { target, tracker, props, sent, notify } = setup({
      ...DEFAULT_INPUT,
      sensitivity: 0.8,
      noiseGate: 0.6,
    });
    expect(sent).toEqual([]);
    // 感度と閾値を既定に戻し、トラッキングのパネルでなめらかさを変える。
    const resets = target.querySelectorAll<HTMLElement>(".ugoe-slider__reset");
    expect(resets).toHaveLength(2);
    resets[0]?.click();
    flushSync();
    resets[1]?.click();
    flushSync();
    tracker.smoothing = 0.9;
    flushSync();
    await notify();
    expect(props.settings).toEqual({
      ...DEFAULT_INPUT,
      sensitivity: 0.5,
      noiseGate: 0.3,
      smoothing: 0.9,
    });
  });
});
