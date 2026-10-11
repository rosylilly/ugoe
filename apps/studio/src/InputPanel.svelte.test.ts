// @vitest-environment happy-dom
// 入力の設定 (InputPanel.svelte): プロファイルの通知を待たずに続けて変えても、変更が消えないこと。
import { FaceTracker } from "@ugoe/face-tracking";
import { setLocale } from "@ugoe/i18n";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import InputPanel from "./InputPanel.svelte";
import { DEFAULT_INPUT, type InputSettings } from "./lib/inputs";
import { Microphone } from "./lib/microphone.svelte";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
  setLocale("system");
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
    // 感度と閾値を既定に戻し、トラッキングのパネルでなめらかさと調整を変える。
    const resets = target.querySelectorAll<HTMLElement>(".ugoe-slider__reset");
    expect(resets).toHaveLength(2);
    resets[0]?.click();
    flushSync();
    resets[1]?.click();
    flushSync();
    tracker.smoothing = 0.9;
    flushSync();
    // 調整もプロファイルに残る。
    tracker.setTuning({ ...DEFAULT_INPUT.tuning, jawOpen: { low: 0.1, high: 0.6 } });
    flushSync();
    await notify();
    expect(props.settings).toEqual({
      ...DEFAULT_INPUT,
      sensitivity: 0.5,
      noiseGate: 0.3,
      smoothing: 0.9,
      tuning: { ...DEFAULT_INPUT.tuning, jawOpen: { low: 0.1, high: 0.6 } },
    });
  });
});

describe("InputPanel tracking tuning", () => {
  it("opens the tuning, saves a change and resets it", async () => {
    setLocale("en");
    const { target, tracker, props, notify } = setup(DEFAULT_INPUT);
    const trigger = [...target.querySelectorAll<HTMLElement>(".ugoe-section__trigger")].find((b) =>
      b.textContent?.includes("Tuning"),
    );
    expect(trigger).toBeDefined();
    trigger?.click();
    flushSync();
    // 小見出しは閉じていて、中身を作らない。
    expect(target.querySelectorAll(".meter-value")).toHaveLength(0);
    openSection(target, "Head");
    // キャリブレーションの前は、値が中立を差し引いていないことを知らせる。動いていないのでメーターは「—」。
    expect(target.textContent).toContain("Not calibrated");
    expect(target.querySelectorAll(".meter-value")[0]?.textContent).toBe("—");
    const reset = [...target.querySelectorAll<HTMLButtonElement>("button")].find(
      (b) => b.textContent === "Reset to defaults",
    );
    expect(reset?.disabled).toBe(true);
    tracker.setTuning({
      ...DEFAULT_INPUT.tuning,
      roll: { ...DEFAULT_INPUT.tuning.roll, gain: 0.5, deadZone: 2 },
    });
    flushSync();
    await notify();
    expect(props.settings.tuning.roll).toEqual({
      ...DEFAULT_INPUT.tuning.roll,
      gain: 0.5,
      deadZone: 2,
    });
    expect(reset?.disabled).toBe(false);
    reset?.click();
    flushSync();
    await notify();
    expect(props.settings.tuning).toEqual(DEFAULT_INPUT.tuning);
  });

  it("separates the left and right eyelids and saves both", async () => {
    setLocale("en");
    const { target, tracker, props, notify } = setup(DEFAULT_INPUT);
    tracker.setTuning({ ...DEFAULT_INPUT.tuning, eyeBlink: { low: 0.1, high: 0.6 } });
    flushSync();
    await notify();
    openSection(target, "Tuning");
    openSection(target, "Eyes");
    expect(target.textContent).toContain("Eyelids (both eyes)");
    const toggle = [...target.querySelectorAll<HTMLElement>("[role=switch]")].find((b) =>
      b.closest(".ugoe-toggle")?.textContent?.includes("Separate left and right eyelids"),
    );
    expect(toggle?.getAttribute("aria-checked")).toBe("false");
    toggle?.click();
    flushSync();
    await notify();
    // 入れると右目に両目の境目を写す。
    expect(props.settings.tuning.eyeBlinkSeparate).toBe(true);
    expect(props.settings.tuning.eyeBlinkRight).toEqual({ low: 0.1, high: 0.6 });
    expect(target.textContent).toContain("Eyelid (left eye)");
    expect(target.textContent).toContain("Eyelid (right eye)");
    expect(target.textContent).not.toContain("Eyelids (both eyes)");
  });

  it("separates the head turn sensitivity by direction and saves both", async () => {
    setLocale("en");
    const { target, tracker, props, notify } = setup(DEFAULT_INPUT);
    tracker.setTuning({ ...DEFAULT_INPUT.tuning, yaw: { ...DEFAULT_INPUT.tuning.yaw, gain: 1.5 } });
    flushSync();
    await notify();
    openSection(target, "Tuning");
    openSection(target, "Head");
    const toggles = [...target.querySelectorAll<HTMLElement>("[role=switch]")].filter((b) =>
      b.closest(".ugoe-toggle")?.textContent?.includes("Separate sensitivity by direction"),
    );
    // 軸ごとに 1 つ (ヨー・ピッチ・ロール)。
    expect(toggles).toHaveLength(3);
    expect(target.querySelector(".meter-ends")).toBeNull();
    toggles[0]?.click();
    flushSync();
    await notify();
    // 入れると負の向きに今の感度を写す。ほかの軸は分けない。
    expect(props.settings.tuning.yaw).toEqual({
      ...DEFAULT_INPUT.tuning.yaw,
      gain: 1.5,
      separate: true,
      gainNegative: 1.5,
    });
    expect(props.settings.tuning.pitch.separate).toBe(false);
    expect(target.textContent).toContain("Sensitivity (left)");
    expect(target.textContent).toContain("Sensitivity (right)");
    // メーターの両端に向きを書く (負の向きが左)。
    expect(target.querySelector(".meter-ends")?.textContent).toBe("rightleft");
  });
});

/** 見出しの名前で折りたたみを開く。 */
function openSection(target: HTMLElement, title: string): void {
  const trigger = [...target.querySelectorAll<HTMLElement>(".ugoe-section__trigger")].find(
    (b) => b.textContent?.trim().startsWith(title) && b.getAttribute("aria-expanded") !== "true",
  );
  expect(trigger).toBeDefined();
  trigger?.click();
  flushSync();
}
