// @vitest-environment happy-dom
// タブの並び (TabStrip): アイコンだけのタブ (右の列。T1632) の読み上げの名前・数と変化の印・Tooltip・キーボード (docs/editor.md#画面の配置)。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import { render } from "../viewport/tools/test/harness.svelte";
import TabStrip, { type TabItem } from "./TabStrip.svelte";

type Tab = "inspector" | "physics" | "ai";

const tabs: TabItem<Tab>[] = [
  { value: "inspector", icon: "inspector", label: "Inspector", tooltip: "Inspector: properties" },
  { value: "physics", icon: "physics", label: "Physics", tooltip: "Physics: pendulum rigs" },
  {
    value: "ai",
    icon: "ai",
    label: "AI (3)",
    tooltip: "AI (3): an agent made a new proposal",
    badge: 3,
    attention: true,
  },
];

function strip(props: Partial<{ value: Tab; icons: boolean; onselect: (v: Tab) => void }> = {}) {
  const { target } = render(TabStrip<Tab>, {
    tabs,
    value: props.value ?? "inspector",
    "aria-label": "Inspector panels",
    attentionLabel: "needs attention",
    icons: props.icons ?? true,
    onselect: props.onselect ?? (() => {}),
  });
  return [...target.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
}

function press(el: HTMLElement, key: string): void {
  el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  flushSync();
}

describe("TabStrip with icons", () => {
  it("names every icon tab and keeps the count and the attention mark", () => {
    const [inspector, physics, ai] = strip();
    expect(inspector?.getAttribute("aria-label")).toBe("Inspector");
    expect(physics?.getAttribute("aria-label")).toBe("Physics");
    expect(ai?.getAttribute("aria-label")).toBe("AI (3) needs attention");
    // 見せるのはアイコンと数と点だけ (名前の文字は出さない)
    expect(inspector?.querySelector("svg.ugoe-icon")).not.toBeNull();
    expect(inspector?.textContent?.trim()).toBe("");
    expect(ai?.querySelector(".badge")?.textContent).toBe("3");
    expect(ai?.querySelector(".dot")).not.toBeNull();
    expect(ai?.classList.contains("attention")).toBe(true);
    for (const tab of [inspector, physics, ai]) expect(tab?.hasAttribute("title")).toBe(false);
  });

  it("is one tab stop and moves with the arrows, Home and End", () => {
    const onselect = vi.fn();
    const [inspector, physics, ai] = strip({ value: "physics", onselect });
    expect([inspector?.tabIndex, physics?.tabIndex, ai?.tabIndex]).toEqual([-1, 0, -1]);
    expect(physics?.getAttribute("aria-selected")).toBe("true");
    if (!physics) throw new Error("no tab");
    press(physics, "ArrowRight");
    expect(onselect).toHaveBeenLastCalledWith("ai");
    expect(document.activeElement).toBe(ai);
    press(physics, "ArrowLeft");
    expect(onselect).toHaveBeenLastCalledWith("inspector");
    press(physics, "End");
    expect(onselect).toHaveBeenLastCalledWith("ai");
    press(physics, "Home");
    expect(onselect).toHaveBeenLastCalledWith("inspector");
  });

  it("shows the name and the description on keyboard focus", async () => {
    const [, physics] = strip();
    physics?.focus();
    const tip = await vi.waitFor(() => {
      const found = document.querySelector('[role="tooltip"]');
      if (!found) throw new Error("no tooltip");
      return found;
    });
    expect(tip.textContent).toContain("Physics: pendulum rigs");
  });

  it("keeps the text labels without icons", () => {
    const [inspector, , ai] = strip({ icons: false });
    expect(inspector?.textContent?.trim()).toBe("Inspector");
    expect(inspector?.hasAttribute("aria-label")).toBe(false);
    expect(ai?.textContent).toContain("needs attention");
  });
});
