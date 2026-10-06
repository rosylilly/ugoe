// @vitest-environment happy-dom
// ツールバーの「背景」(BackgroundPopover.svelte): 背景の色と「透明」を Popover にまとめ、
// Esc で閉じると開いたボタンへフォーカスが戻ること (T0737)。
import { setLocale } from "@ugoe/i18n";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import BackgroundPopover from "./BackgroundPopover.svelte";

const cleanups: (() => void)[] = [];

beforeEach(() => setLocale("ja"));

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

const focused = () => document.activeElement as HTMLElement | null;
const popovers = () => [...document.querySelectorAll<HTMLElement>(".ugoe-popover")];

function press(el: Element, key: string) {
  el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  flushSync();
}

function click(el: Element) {
  (el as HTMLElement).click();
  flushSync();
}

/** 「背景」を描く。`onchange` は受け取った値を props に書き戻す (App.svelte が Player を通して行うのと同じ)。 */
function setup(color = "#336699", transparent = false) {
  const props = $state({ color, transparent, onchange: vi.fn() });
  props.onchange.mockImplementation((c: string, tr: boolean) => {
    props.color = c;
    props.transparent = tr;
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(BackgroundPopover, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  const trigger = target.querySelector<HTMLElement>("button");
  if (!trigger) throw new Error("no trigger");
  return { target, trigger, props };
}

async function open(trigger: HTMLElement) {
  trigger.focus();
  click(trigger);
  await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
  const panel = popovers()[0];
  if (!panel) throw new Error("no popover");
  await vi.waitFor(() => expect(panel.contains(focused())).toBe(true));
  return panel;
}

describe("BackgroundPopover", () => {
  it("shows one button with the current background", () => {
    const { target, trigger, props } = setup();
    expect(target.querySelectorAll("button")).toHaveLength(1);
    expect(trigger.textContent?.trim()).toBe("背景");
    expect(trigger.getAttribute("aria-haspopup")).toBe("dialog");
    const swatch = target.querySelector<HTMLElement>(".swatch");
    expect(swatch?.style.background).toContain("#336699");
    props.transparent = true;
    flushSync();
    expect(swatch?.classList.contains("transparent")).toBe(true);
  });

  it("holds the color and the transparent switch, and closes with Escape back to the button", async () => {
    const { trigger } = setup();
    const panel = await open(trigger);
    expect(panel.getAttribute("aria-label")).toBe("背景");
    expect(panel.textContent).toContain("色");
    expect(panel.querySelector('[role="switch"]')).not.toBeNull();
    press(focused() as HTMLElement, "Escape");
    await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("false"));
    await vi.waitFor(() => expect(focused()).toBe(trigger));
  });

  it("changes the transparency and keeps the color", async () => {
    const { trigger, props } = setup();
    const panel = await open(trigger);
    click(panel.querySelector('[role="switch"]') as HTMLElement);
    expect(props.onchange).toHaveBeenLastCalledWith("#336699", true);
    expect(trigger.querySelector(".swatch.transparent")).not.toBeNull();
  });

  it("changes the color and keeps the transparency", async () => {
    const { trigger, props } = setup("#336699", true);
    const panel = await open(trigger);
    const colorTrigger = panel.querySelector<HTMLElement>(".ugoe-color-picker button");
    if (!colorTrigger) throw new Error("no color trigger");
    click(colorTrigger);
    const inner = await vi.waitFor(() => {
      const p = popovers().find((p) => p !== panel);
      if (!p) throw new Error("no color popover");
      return p;
    });
    const native = inner.querySelector<HTMLInputElement>('input[type="color"]');
    if (!native) throw new Error("no color input");
    native.value = "#ff0000";
    native.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    expect(props.onchange).toHaveBeenLastCalledWith("#ff0000", true);
    // 中の色の枠を Esc で閉じても、「背景」の枠は開いたまま
    await vi.waitFor(() => expect(inner.contains(focused())).toBe(true));
    press(focused() as HTMLElement, "Escape");
    await vi.waitFor(() => expect(colorTrigger.getAttribute("aria-expanded")).toBe("false"));
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
  });
});
