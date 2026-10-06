// 一覧が送るときだけ行末をスクロールバーの分空ける印 (scrollGap.ts。T0976)。
// happy-dom は大きさを測らないので、枠と中身の高さを差し替える。

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ContextMenu from "./components/ContextMenu.svelte";
import Menu from "./components/Menu.svelte";
import Select from "./components/Select.svelte";
import { markScrolling } from "./scrollGap";
import { get, html, press, render } from "./test/render";
import type { MenuEntry } from "./types";

/** 送る枠 (この印を付けた要素) の中身の高さ。枠の高さは 100 にする。 */
let contentHeight = 0;
const SCROLLERS = ".ugoe-menu, .ugoe-select__content, .scroller";

beforeEach(() => {
  contentHeight = 0;
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockImplementation(function (
    this: HTMLElement,
  ) {
    return this.matches(SCROLLERS) ? 100 : 0;
  });
  vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockImplementation(function (
    this: HTMLElement,
  ) {
    return this.matches(SCROLLERS) ? contentHeight : 0;
  });
});

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
  vi.restoreAllMocks();
});

describe("markScrolling", () => {
  function setup() {
    const el = document.createElement("div");
    el.className = "scroller";
    el.append(document.createElement("div"));
    document.body.append(el);
    const cleanup = markScrolling(el);
    cleanups.push(() => {
      cleanup?.();
      el.remove();
    });
    return el;
  }

  it("marks the element only while its content is taller than the box", () => {
    contentHeight = 100;
    const el = setup();
    expect(el.hasAttribute("data-scrolling")).toBe(false);
  });

  it("marks a scrolling element", () => {
    contentHeight = 160;
    const el = setup();
    expect(el.hasAttribute("data-scrolling")).toBe(true);
  });

  it("ignores a one pixel difference from rounding", () => {
    contentHeight = 101;
    const el = setup();
    expect(el.hasAttribute("data-scrolling")).toBe(false);
  });

  it("measures again when items are added or removed", async () => {
    const el = setup();
    expect(el.hasAttribute("data-scrolling")).toBe(false);
    contentHeight = 400;
    el.append(document.createElement("div"));
    await vi.waitFor(() => expect(el.hasAttribute("data-scrolling")).toBe(true));
    contentHeight = 50;
    el.lastElementChild?.remove();
    await vi.waitFor(() => expect(el.hasAttribute("data-scrolling")).toBe(false));
  });
});

const entries: MenuEntry[] = [
  { label: "Save", shortcut: "Mod+S", onselect: () => {} },
  { label: "Close", shortcut: "Mod+W", onselect: () => {} },
];

async function openMenu() {
  const root = render(Menu, { label: "File", entries });
  const trigger = get(root, "button");
  trigger.focus();
  press(trigger, "ArrowDown");
  await vi.waitFor(() => expect(document.querySelector('[role="menu"]')).not.toBeNull());
  return get(document, ".ugoe-menu");
}

async function openSelect() {
  const options = ["Alpha", "Beta", "Gamma"].map((label) => ({ value: label, label }));
  const root = render(Select, { label: "Camera", options, value: "Alpha" });
  const trigger = get(root, '[aria-haspopup="listbox"]');
  trigger.focus();
  press(trigger, "ArrowDown");
  await vi.waitFor(() => expect(document.querySelector(".ugoe-select__content")).not.toBeNull());
  return get(document, ".ugoe-select__content");
}

describe.each([
  ["Menu", openMenu],
  ["Select", openSelect],
])("%s list", (_name, open) => {
  it("leaves the scrollbar gap only while it scrolls", async () => {
    contentHeight = 400;
    const list = await open();
    await vi.waitFor(() => expect(list.hasAttribute("data-scrolling")).toBe(true));
  });

  it("keeps its width when it does not scroll", async () => {
    contentHeight = 80;
    const list = await open();
    expect(list.hasAttribute("data-scrolling")).toBe(false);
  });
});

describe("ContextMenu list", () => {
  it("leaves the scrollbar gap while it scrolls", async () => {
    contentHeight = 400;
    const root = render(ContextMenu, { entries, label: "Layer", children: html("<p>Layer</p>") });
    const area = get(root, ".ugoe-context-menu__area");
    area.dispatchEvent(
      new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 5, clientY: 5 }),
    );
    await vi.waitFor(() => expect(document.querySelector(".ugoe-menu")).not.toBeNull());
    const list = get(document, ".ugoe-menu");
    await vi.waitFor(() => expect(list.hasAttribute("data-scrolling")).toBe(true));
  });
});
