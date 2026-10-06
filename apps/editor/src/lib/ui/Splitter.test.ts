// @vitest-environment happy-dom
// 区切り (Splitter): キーボードで大きさを変えて覚え、Enter で既定に戻す (docs/editor.md#画面の配置、docs/ui.md#レイアウト の「分割」)。
import { MemoryStorage, useLocalStorage } from "@ugoe/i18n/testing";
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import { LEFT_WIDTH, RIGHT_SPLIT, TIMELINE_HEIGHT } from "../layout";
import { render } from "../viewport/tools/test/harness.svelte";
import Splitter from "./Splitter.svelte";

function separator(target: HTMLElement): HTMLElement {
  const found = target.querySelector<HTMLElement>('[role="separator"]');
  if (!found) throw new Error("no separator");
  return found;
}

function press(el: HTMLElement, key: string): void {
  el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  flushSync();
}

describe("Splitter", () => {
  it("is a focusable separator with its range and value", () => {
    const { target } = render(Splitter, {
      label: "Resize the tree column",
      orientation: "vertical",
      range: LEFT_WIDTH,
      value: 240,
    });
    const el = separator(target);
    expect(el.tabIndex).toBe(0);
    expect(el.getAttribute("aria-label")).toBe("Resize the tree column");
    expect(el.getAttribute("aria-orientation")).toBe("vertical");
    expect(el.getAttribute("aria-valuenow")).toBe("240");
    expect(el.getAttribute("aria-valuemin")).toBe(String(LEFT_WIDTH.min));
    expect(el.getAttribute("aria-valuemax")).toBe(String(LEFT_WIDTH.max));
  });

  it("resizes with the arrows along its line, remembers it and resets with Enter", () => {
    const storage = new MemoryStorage();
    useLocalStorage(storage);
    const { target } = render(Splitter, {
      label: "Resize",
      orientation: "vertical",
      range: LEFT_WIDTH,
      value: 240,
    });
    const el = separator(target);
    press(el, "ArrowRight");
    expect(el.getAttribute("aria-valuenow")).toBe(String(240 + LEFT_WIDTH.step));
    expect(storage.getItem(LEFT_WIDTH.key)).toBe(String(240 + LEFT_WIDTH.step));
    press(el, "ArrowUp");
    expect(el.getAttribute("aria-valuenow")).toBe(String(240 + LEFT_WIDTH.step));
    press(el, "End");
    expect(el.getAttribute("aria-valuenow")).toBe(String(LEFT_WIDTH.max));
    press(el, "Enter");
    expect(el.getAttribute("aria-valuenow")).toBe(String(LEFT_WIDTH.default));
    expect(storage.getItem(LEFT_WIDTH.key)).toBeNull();
  });

  it("starts from the measured size when the size comes from CSS", () => {
    useLocalStorage(new MemoryStorage());
    const { target } = render(Splitter, {
      label: "Resize the timeline",
      orientation: "horizontal",
      reverse: true,
      range: TIMELINE_HEIGHT,
      value: null,
      measure: () => 300,
    });
    const el = separator(target);
    expect(el.getAttribute("aria-valuenow")).toBe("300");
    press(el, "ArrowUp");
    expect(el.getAttribute("aria-valuenow")).toBe(String(300 + TIMELINE_HEIGHT.step));
  });

  it("reads a ratio as a percentage", () => {
    useLocalStorage(new MemoryStorage());
    const { target } = render(Splitter, {
      label: "Resize",
      orientation: "horizontal",
      range: RIGHT_SPLIT,
      value: 0.6,
    });
    const el = separator(target);
    expect(el.getAttribute("aria-valuenow")).toBe("60");
    expect(el.getAttribute("aria-valuemin")).toBe("15");
    press(el, "ArrowDown");
    expect(el.getAttribute("aria-valuenow")).toBe("65");
  });
});
