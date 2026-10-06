// @vitest-environment happy-dom
// 「いまの状態」の印 (StatusBadge.svelte) と、印からパネルを開いて送る (lib/status.ts の revealPanel。T0845)。
import { Section } from "@ugoe/ui";
import { createRawSnippet, flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import { EMPTY_API } from "./lib/api";
import { t } from "./lib/i18n";
import { hotkeysBadge, revealPanel, STATUS_PANELS, statusBadges } from "./lib/status";
import StatusBadge from "./StatusBadge.svelte";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

function place(): HTMLElement {
  const target = document.createElement("div");
  document.body.append(target);
  cleanups.push(() => target.remove());
  return target;
}

describe("StatusBadge", () => {
  it("is a button that opens its panel when given onopen", () => {
    const target = place();
    const opened: string[] = [];
    const view = hotkeysBadge({ count: 2, problems: 1, wayland: false });
    const instance = mount(StatusBadge, {
      target,
      props: { view, onopen: () => opened.push(view.panel) },
    });
    cleanups.push(() => unmount(instance));
    flushSync();
    const badge = target.querySelector("button");
    expect(badge?.textContent).toContain(t("status.hotkeys.problems", { count: 1 }));
    // ボタンなので、Enter・Space でも押せる (キーボードの既定の動き)
    expect(badge?.getAttribute("type")).toBe("button");
    badge?.click();
    expect(opened).toEqual(["hotkeys"]);
  });

  it("stays a focusable mark without onopen", () => {
    const target = place();
    const instance = mount(StatusBadge, {
      target,
      props: { view: hotkeysBadge({ count: 0, problems: 0, wayland: false }) },
    });
    cleanups.push(() => unmount(instance));
    flushSync();
    expect(target.querySelector("button")).toBeNull();
    expect(target.querySelector(".status-badge")?.getAttribute("tabindex")).toBe("0");
  });
});

describe("revealPanel", () => {
  it("maps every badge to a settings panel", () => {
    expect(STATUS_PANELS).toEqual({
      tracking: "inputs",
      lipSync: "inputs",
      hotkeys: "hotkeys",
      output: "output",
      load: "load",
      api: "api",
      source: "source",
    });
    const badge = statusBadges({
      tracking: { status: "idle", faceFound: false, fps: 0, paused: false },
      lipSync: { running: false, paused: false },
      hotkeys: { count: 0, problems: 0, wayland: false },
      output: null,
      load: null,
      api: EMPTY_API,
    });
    expect(badge.map((b) => b.panel)).toEqual(["inputs", "inputs", "hotkeys", "api", "source"]);
  });

  it("opens a closed panel, scrolls to it and focuses its heading", async () => {
    const target = place();
    // 送れる右の列 (中身が高さより長い)
    const column = document.createElement("aside");
    column.style.overflowY = "auto";
    Object.defineProperty(column, "scrollHeight", { value: 1000 });
    Object.defineProperty(column, "clientHeight", { value: 300 });
    const scrolls: ScrollToOptions[] = [];
    column.scrollTo = ((options: ScrollToOptions) =>
      scrolls.push(options)) as typeof column.scrollTo;
    const container = document.createElement("div");
    container.dataset.panel = "hotkeys";
    container.getBoundingClientRect = () => new DOMRect(0, 640, 300, 200);
    column.append(container);
    target.append(column);
    const props = $state({
      title: "Hotkeys",
      open: false,
      children: createRawSnippet(() => ({ render: () => "<p>rows</p>" })),
    });
    const instance = mount(Section, { target: container, props });
    cleanups.push(() => unmount(instance));
    flushSync();
    const trigger = container.querySelector(".ugoe-section__trigger");
    expect(trigger?.getAttribute("aria-expanded")).toBe("false");

    const found = await revealPanel(target, "hotkeys", async () => {
      props.open = true;
      flushSync();
    });
    expect(found).toBe(true);
    expect(trigger?.getAttribute("aria-expanded")).toBe("true");
    expect(scrolls.map((o) => o.top)).toEqual([640]);
    expect(document.activeElement).toBe(trigger);
  });

  it("reports a missing panel", async () => {
    const target = place();
    expect(await revealPanel(target, "load", async () => {})).toBe(false);
  });
});
