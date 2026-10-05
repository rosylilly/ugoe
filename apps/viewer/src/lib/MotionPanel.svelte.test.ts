// @vitest-environment happy-dom
// モーションのパネル (MotionPanel.svelte): イベントの欄に、発火したモーションの名前と番号を出すこと (T0734)。
import { setLocale } from "@ugoe/i18n";
import type { MotionEventInfo, Player } from "@ugoe/player";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { t } from "./i18n";
import MotionPanel from "./MotionPanel.svelte";

const cleanups: (() => void)[] = [];

beforeEach(() => setLocale("ja"));

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

/** パネルが読む Player の状態だけを持つ偽物 (`motionEvents` はテストが書き換える)。 */
function setup() {
  const player = $state({
    motions: [
      { name: "idle", duration: 1, looping: true, fadeIn: 0, fadeOut: 0, parameters: [] },
      { name: "wave", duration: 1, looping: false, fadeIn: 0, fadeOut: 0, parameters: [] },
    ],
    expressions: [],
    activeExpressions: [],
    currentMotion: "wave",
    motionPriority: "normal",
    repeatMotion: false,
    autoIdle: true,
    idleMotion: "idle",
    idleRemembered: false,
    motionNotice: null,
    motionOverTracking: false,
    tracking: false,
    trackingParameterIds: [],
    trackingKept: [],
    motionEvents: [] as MotionEventInfo[],
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(MotionPanel, { target, props: { player: player as unknown as Player } });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  return { target, player };
}

describe("MotionPanel", () => {
  it("shows the motion and the number that fired each event", () => {
    const { target, player } = setup();
    expect(target.querySelector(".events")).toBeNull();

    // クロスフェード中の 2 つと、同じ名前を始め直したもの (新しい順)。
    player.motionEvents = [
      { value: "hi", time: 0, motion: "wave", number: 3 },
      { value: "hi", time: 0, motion: "wave", number: 2 },
      { value: "blink", time: 0.5, motion: "idle", number: 1 },
    ];
    flushSync();
    const events = target.querySelector(".events");
    expect(events?.textContent?.trim()).toBe(
      "イベント: hi (wave #3), hi (wave #2), blink (idle #1)",
    );
    // 読み方の説明は title でなく見える補足に出す
    expect(events?.hasAttribute("title")).toBe(false);
    expect(target.querySelector(".events-description")?.textContent).toBe(
      t("motions.eventsDescription"),
    );
  });
});
