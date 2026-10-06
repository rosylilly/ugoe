import { describe, expect, it } from "vitest";
import { acceptsNudgeKey, acceptsToolKey, NUDGE, NUDGE_LARGE, nudgeOf } from "./keys";

/** `closest` が `selector` のどれかに当たるかだけを返す偽の要素。 */
function at(matches: string | null): EventTarget {
  return {
    closest: (selector: string) => (matches !== null && selector.includes(matches) ? {} : null),
  } as unknown as EventTarget;
}

describe("acceptsToolKey", () => {
  it("accepts keys on the page and the viewport", () => {
    expect(acceptsToolKey(at(null))).toBe(true);
    expect(acceptsToolKey(null)).toBe(true);
  });

  it("ignores keys in text fields and dialogs", () => {
    expect(acceptsToolKey(at("input"))).toBe(false);
    expect(acceptsToolKey(at("[contenteditable]"))).toBe(false);
    expect(acceptsToolKey(at('[role="dialog"]'))).toBe(false);
    expect(acceptsToolKey(at('[role="alertdialog"]'))).toBe(false);
  });
});

describe("acceptsNudgeKey", () => {
  it("accepts arrows on the page and in the viewport", () => {
    expect(acceptsNudgeKey(null)).toBe(true);
    expect(acceptsNudgeKey({ tagName: "BODY" } as unknown as EventTarget)).toBe(true);
    expect(acceptsNudgeKey(at("[data-viewport]"))).toBe(true);
  });

  it("leaves arrows to the tree, sliders and the timeline", () => {
    // ビューポートの外のボタン・木の項目 (closest が何にも当たらない)
    expect(acceptsNudgeKey(at(null))).toBe(false);
    expect(acceptsNudgeKey(at("input"))).toBe(false);
    expect(acceptsNudgeKey(at('[role="dialog"]'))).toBe(false);
  });
});

describe("nudgeOf", () => {
  const key = (
    k: string,
    mods: Partial<Record<"shiftKey" | "ctrlKey" | "metaKey" | "altKey", boolean>> = {},
  ) => ({
    key: k,
    shiftKey: false,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    ...mods,
  });

  it("moves by one pixel, or ten with Shift", () => {
    expect(nudgeOf(key("ArrowLeft"))).toEqual([-NUDGE, 0]);
    expect(nudgeOf(key("ArrowDown"))).toEqual([0, NUDGE]);
    expect(nudgeOf(key("ArrowUp", { shiftKey: true }))).toEqual([0, -NUDGE_LARGE]);
    expect(nudgeOf(key("ArrowRight", { shiftKey: true }))).toEqual([NUDGE_LARGE, 0]);
  });

  it("ignores other keys and arrows with Ctrl, Cmd or Alt", () => {
    expect(nudgeOf(key("a"))).toBeNull();
    expect(nudgeOf(key("ArrowLeft", { ctrlKey: true }))).toBeNull();
    expect(nudgeOf(key("ArrowLeft", { metaKey: true }))).toBeNull();
    expect(nudgeOf(key("ArrowLeft", { altKey: true }))).toBeNull();
  });
});
