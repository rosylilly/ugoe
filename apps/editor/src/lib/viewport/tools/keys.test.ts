import { describe, expect, it } from "vitest";
import {
  acceptsNudgeKey,
  acceptsToolKey,
  extendedPoints,
  isSplitKey,
  NUDGE,
  NUDGE_LARGE,
  nudgeOf,
  PAN,
  PAN_LARGE,
  panOf,
  pointExtendOf,
  pointStepOf,
  REVEAL_MARGIN,
  revealPanOf,
  steppedPoint,
} from "./keys";

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

describe("nudgeOf", () => {
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

describe("panOf", () => {
  it("pans with Alt + arrows, farther with Shift, moving the picture like Alt + drag", () => {
    expect(panOf(key("ArrowRight", { altKey: true }))).toEqual([PAN, 0]);
    expect(panOf(key("ArrowUp", { altKey: true }))).toEqual([0, -PAN]);
    expect(panOf(key("ArrowLeft", { altKey: true, shiftKey: true }))).toEqual([-PAN_LARGE, 0]);
    expect(panOf(key("ArrowDown", { altKey: true, shiftKey: true }))).toEqual([0, PAN_LARGE]);
  });

  it("leaves plain arrows to the point nudge and ignores Ctrl, Cmd and other keys", () => {
    expect(panOf(key("ArrowRight"))).toBeNull();
    expect(panOf(key("ArrowRight", { shiftKey: true }))).toBeNull();
    expect(panOf(key("ArrowRight", { altKey: true, ctrlKey: true }))).toBeNull();
    expect(panOf(key("ArrowRight", { altKey: true, metaKey: true }))).toBeNull();
    expect(panOf(key("a", { altKey: true }))).toBeNull();
  });
});

describe("pointStepOf", () => {
  it("steps forward with ] and back with [", () => {
    expect(pointStepOf(key("]"))).toBe(1);
    expect(pointStepOf(key("["))).toBe(-1);
  });

  it("ignores Tab, other keys and modifiers", () => {
    expect(pointStepOf(key("Tab"))).toBeNull();
    expect(pointStepOf(key("}"))).toBeNull();
    expect(pointStepOf(key("]", { ctrlKey: true }))).toBeNull();
    expect(pointStepOf(key("[", { metaKey: true }))).toBeNull();
    expect(pointStepOf(key("[", { altKey: true }))).toBeNull();
  });
});

describe("steppedPoint", () => {
  it("starts at the first point forward and the last point backward", () => {
    expect(steppedPoint([], 4, 1)).toBe(0);
    expect(steppedPoint([], 4, -1)).toBe(3);
  });

  it("goes round from the last picked point", () => {
    expect(steppedPoint([1], 4, 1)).toBe(2);
    expect(steppedPoint([1], 4, -1)).toBe(0);
    expect(steppedPoint([3], 4, 1)).toBe(0);
    expect(steppedPoint([0], 4, -1)).toBe(3);
    // 複数を選んでいれば最後に選んだ点から
    expect(steppedPoint([0, 2], 4, 1)).toBe(3);
  });

  it("starts over when the picked point is gone, and has nothing to pick without points", () => {
    expect(steppedPoint([9], 4, 1)).toBe(0);
    expect(steppedPoint([], 0, 1)).toBeNull();
    expect(steppedPoint([0], 0, -1)).toBeNull();
  });
});

describe("pointExtendOf", () => {
  it("picks a second point with Shift + ] and Shift + [ on any layout", () => {
    expect(pointExtendOf(key("}", { shiftKey: true }))).toBe(1);
    expect(pointExtendOf(key("{", { shiftKey: true }))).toBe(-1);
    expect(pointExtendOf(key("]", { shiftKey: true }))).toBe(1);
    expect(pointExtendOf(key("[", { shiftKey: true }))).toBe(-1);
  });

  it("ignores ] [ without Shift and with other modifiers", () => {
    expect(pointExtendOf(key("]"))).toBeNull();
    expect(pointExtendOf(key("}", { shiftKey: true, ctrlKey: true }))).toBeNull();
    expect(pointExtendOf(key("{", { shiftKey: true, altKey: true }))).toBeNull();
    expect(pointExtendOf(key("A", { shiftKey: true }))).toBeNull();
  });
});

describe("extendedPoints", () => {
  it("keeps the first point and steps the second from it", () => {
    expect(extendedPoints([1], 4, 1)).toEqual([1, 2]);
    expect(extendedPoints([1], 4, -1)).toEqual([1, 0]);
    expect(extendedPoints([1, 2], 4, 1)).toEqual([1, 3]);
    // 3 つ以上選んでいても、先頭と末尾から 2 つにする
    expect(extendedPoints([0, 3, 2], 4, -1)).toEqual([0, 1]);
  });

  it("skips the first point when going round", () => {
    expect(extendedPoints([0, 3], 4, 1)).toEqual([0, 1]);
    expect(extendedPoints([2, 3], 4, 1)).toEqual([2, 0]);
    expect(extendedPoints([1, 2], 4, -1)).toEqual([1, 0]);
  });

  it("picks one point like ] [ when nothing is picked", () => {
    expect(extendedPoints([], 4, 1)).toEqual([0]);
    expect(extendedPoints([9], 4, -1)).toEqual([3]);
    expect(extendedPoints([0], 1, 1)).toEqual([0]);
    expect(extendedPoints([], 0, 1)).toBeNull();
  });
});

describe("isSplitKey", () => {
  it("is I without modifiers", () => {
    expect(isSplitKey(key("i"))).toBe(true);
    expect(isSplitKey(key("I"))).toBe(true);
    expect(isSplitKey(key("i", { ctrlKey: true }))).toBe(false);
    expect(isSplitKey(key("I", { shiftKey: true }))).toBe(false);
    expect(isSplitKey(key("i", { metaKey: true }))).toBe(false);
    expect(isSplitKey(key("j"))).toBe(false);
  });
});

describe("revealPanOf", () => {
  const m = REVEAL_MARGIN;

  it("leaves a point inside the margins where it is", () => {
    expect(revealPanOf([400, 300], [800, 600])).toBeNull();
    expect(revealPanOf([m, 600 - m], [800, 600])).toBeNull();
  });

  it("pans a point outside the view or in the margin just inside the edge", () => {
    // 左と上の外
    expect(revealPanOf([-100, -20], [800, 600])).toEqual([100 + m, 20 + m]);
    // 右の外 (縦はそのまま)
    expect(revealPanOf([900, 300], [800, 600])).toEqual([800 - m - 900, 0]);
    // 下の余白の中
    expect(revealPanOf([400, 590], [800, 600])).toEqual([0, 600 - m - 590]);
  });

  it("centers the point on an axis narrower than both margins", () => {
    expect(revealPanOf([0, 300], [60, 600])).toEqual([30, 0]);
    expect(revealPanOf([30, 300], [60, 600])).toBeNull();
  });

  it("does nothing for non-numbers or an empty view", () => {
    expect(revealPanOf([Number.NaN, 0], [800, 600])).toBeNull();
    expect(revealPanOf([0, Number.POSITIVE_INFINITY], [800, 600])).toBeNull();
    expect(revealPanOf([-10, -10], [0, 0])).toBeNull();
    expect(revealPanOf([-10, -10], [800, Number.NaN])).toBeNull();
  });
});
