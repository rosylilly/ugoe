// 部品のキーボード操作と aria の検査 (docs/ui.md#アクセシビリティの基準)。happy-dom の上で描く。

import { setLocale } from "@ugoe/i18n";
import { createRawSnippet, flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import { numberFieldCommit } from "./commit";
import Button from "./components/Button.svelte";
import Checkbox from "./components/Checkbox.svelte";
import IconButton from "./components/IconButton.svelte";
import NumberField from "./components/NumberField.svelte";
import Section from "./components/Section.svelte";
import SegmentedControl from "./components/SegmentedControl.svelte";
import Select from "./components/Select.svelte";
import Slider from "./components/Slider.svelte";
import Tabs from "./components/Tabs.svelte";
import Toggle from "./components/Toggle.svelte";
import ExternalValueHarness from "./test/ExternalValueHarness.svelte";
import { click, get, html, labelledByText, press, render } from "./test/render";
import ToolbarHarness from "./test/ToolbarHarness.svelte";

const icon = html('<svg viewBox="0 0 16 16"></svg>');
const options = [
  { value: "a", label: "Alpha" },
  { value: "b", label: "Beta" },
  { value: "c", label: "Gamma", disabled: true },
  { value: "d", label: "Delta" },
];

describe("Button", () => {
  it("is a plain button with the variant class", () => {
    const onclick = vi.fn();
    const root = render(Button, {
      variant: "primary",
      onclick,
      children: html("<span>Save</span>"),
    });
    const button = get<HTMLButtonElement>(root, "button");
    expect(button.type).toBe("button");
    expect(button.classList).toContain("ugoe-button--primary");
    click(button);
    expect(onclick).toHaveBeenCalledOnce();
  });

  it("does not fire when disabled", () => {
    const onclick = vi.fn();
    const root = render(Button, { disabled: true, onclick, children: html("<span>Save</span>") });
    click(get(root, "button"));
    expect(onclick).not.toHaveBeenCalled();
  });

  it("exposes the pressed state", () => {
    const root = render(Button, { pressed: true, children: html("<span>Mirror</span>") });
    expect(get(root, "button").getAttribute("aria-pressed")).toBe("true");
  });
});

describe("IconButton", () => {
  it("always has an accessible name and hides the icon", () => {
    const root = render(IconButton, { label: "Reset", children: icon });
    const button = get(root, "button");
    expect(button.getAttribute("aria-label")).toBe("Reset");
    expect(button.hasAttribute("title")).toBe(false);
    expect(get(root, ".ugoe-icon-button__icon").getAttribute("aria-hidden")).toBe("true");
  });

  it("complains about an empty label", () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    render(IconButton, { label: " ", children: icon });
    expect(error).toHaveBeenCalled();
    error.mockRestore();
  });
});

describe("Checkbox", () => {
  it("is labelled and toggles", () => {
    const onchange = vi.fn();
    const root = render(Checkbox, { label: "Loop", onchange });
    const box = get(root, '[role="checkbox"]');
    const label = get<HTMLLabelElement>(root, "label");
    expect(label.htmlFor).toBe(box.id);
    expect(label.textContent).toBe("Loop");
    expect(box.getAttribute("aria-checked")).toBe("false");
    click(box);
    expect(box.getAttribute("aria-checked")).toBe("true");
    expect(onchange).toHaveBeenLastCalledWith(true);
  });

  it("shows the mixed state", () => {
    const root = render(Checkbox, { label: "All", indeterminate: true });
    expect(get(root, '[role="checkbox"]').getAttribute("aria-checked")).toBe("mixed");
  });
});

describe("Toggle", () => {
  it("is a labelled switch", () => {
    const onchange = vi.fn();
    const root = render(Toggle, { label: "Mirror", checked: true, onchange });
    const sw = get(root, '[role="switch"]');
    expect(get<HTMLLabelElement>(root, "label").htmlFor).toBe(sw.id);
    expect(sw.getAttribute("aria-checked")).toBe("true");
    click(sw);
    expect(sw.getAttribute("aria-checked")).toBe("false");
    expect(onchange).toHaveBeenLastCalledWith(false);
  });
});

describe("Toggle with a tooltip", () => {
  it("keeps its own description next to the tooltip's", async () => {
    const note = document.createElement("p");
    note.id = "mirror-note";
    note.textContent = "Flips the model";
    document.body.append(note);
    const root = render(Toggle, {
      label: "Mirror",
      tooltip: "Mirror the view",
      describedby: "mirror-note",
    });
    const sw = get(root, '[role="switch"]');
    expect(get<HTMLLabelElement>(root, "label").htmlFor).toBe(sw.id);
    expect(sw.getAttribute("aria-describedby")).toBe("mirror-note");
    sw.focus();
    await vi.waitFor(() => expect(sw.getAttribute("aria-describedby")?.split(" ")).toHaveLength(2));
    const ids = sw.getAttribute("aria-describedby")?.split(" ") ?? [];
    expect(ids).toContain("mirror-note");
    expect(ids.map((id) => document.getElementById(id)?.textContent).join(" ")).toContain(
      "Mirror the view",
    );
    note.remove();
  });
});

describe("NumberField", () => {
  function setup(props: Record<string, unknown> = {}) {
    const onchange = vi.fn();
    const root = render(NumberField, {
      label: "Angle",
      value: 1,
      min: -10,
      max: 10,
      step: 0.5,
      unit: "deg",
      onchange,
      ...props,
    });
    const input = get<HTMLInputElement>(root, "input");
    return { root, input, onchange };
  }

  it("is a labelled spinbutton", () => {
    const { input } = setup();
    expect(input.getAttribute("role")).toBe("spinbutton");
    expect(labelledByText(input)).toBe("Angle");
    expect(input.getAttribute("aria-valuenow")).toBe("1");
    expect(input.getAttribute("aria-valuemin")).toBe("-10");
    expect(input.getAttribute("aria-valuemax")).toBe("10");
    expect(input.getAttribute("aria-valuetext")).toBe("1 deg");
    expect(input.value).toBe("1");
  });

  it("steps with the arrow keys and Shift", () => {
    const { input, onchange } = setup();
    press(input, "ArrowUp");
    expect(input.value).toBe("1.5");
    expect(onchange).toHaveBeenLastCalledWith(1.5);
    press(input, "ArrowDown", { shiftKey: true });
    expect(input.value).toBe("-3.5");
    for (let i = 0; i < 3; i++) press(input, "PageUp");
    expect(input.value).toBe("10");
    expect(input.getAttribute("aria-valuenow")).toBe("10");
  });

  it("commits typed text on Enter and clamps it", () => {
    const { input, onchange } = setup();
    input.focus();
    input.value = "42";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    press(input, "Enter");
    expect(onchange).toHaveBeenLastCalledWith(10);
    expect(input.value).toBe("10");
  });

  it("reverts on Escape and on unreadable input", () => {
    const { input, onchange } = setup();
    input.focus();
    input.value = "7";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    const esc = press(input, "Escape");
    expect(esc.defaultPrevented).toBe(true);
    expect(input.value).toBe("1");
    // 戻すものが無い Esc は外側に渡す
    expect(press(input, "Escape").defaultPrevented).toBe(false);
    input.value = "abc";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    input.blur();
    flushSync();
    expect(input.value).toBe("1");
    expect(onchange).not.toHaveBeenCalled();
  });

  it("marks its commits with a per-field token only while onchange runs", () => {
    const seen: (string | null)[] = [];
    const record = () => seen.push(numberFieldCommit());
    const a = setup({ onchange: record }).input;
    const b = setup({ onchange: record }).input;
    press(a, "ArrowUp");
    press(a, "ArrowUp");
    press(b, "ArrowUp");
    const [first, second, other] = seen;
    expect(first).toEqual(expect.any(String));
    expect(second).toBe(first);
    expect(other).not.toBe(first);
    expect(numberFieldCommit()).toBeNull();
  });

  it("steps from a number when the value is not one", () => {
    const { input, onchange } = setup({ value: Number.NaN, min: 2 });
    expect(input.value).toBe("");
    press(input, "ArrowUp");
    expect(onchange).toHaveBeenLastCalledWith(2.5);
    expect(input.value).toBe("2.5");
  });
});

describe("Slider", () => {
  function setup() {
    const onchange = vi.fn();
    const oncommit = vi.fn();
    const root = render(Slider, {
      label: "Smoothing",
      value: 0.5,
      min: 0,
      max: 1,
      step: 0.1,
      defaultValue: 0.3,
      resetLabel: "Reset to default",
      onchange,
      oncommit,
    });
    return { root, onchange, oncommit };
  }

  it("labels the thumb and the number field with the visible label", () => {
    const { root } = setup();
    const thumb = get(root, '[role="slider"]');
    expect(labelledByText(thumb)).toBe("Smoothing");
    expect(thumb.getAttribute("aria-valuenow")).toBe("0.5");
    expect(labelledByText(get(root, '[role="spinbutton"]'))).toBe("Smoothing");
    expect(get(root, "button").getAttribute("aria-label")).toBe("Reset to default");
  });

  it("moves with the keyboard and keeps the field in sync", () => {
    const { root, onchange, oncommit } = setup();
    const thumb = get(root, '[role="slider"]');
    press(thumb, "ArrowRight");
    expect(thumb.getAttribute("aria-valuenow")).toBe("0.6");
    expect(onchange).toHaveBeenLastCalledWith(0.6);
    expect(oncommit).toHaveBeenLastCalledWith(0.6);
    expect(get<HTMLInputElement>(root, "input").value).toBe("0.6");
    press(thumb, "End");
    expect(thumb.getAttribute("aria-valuenow")).toBe("1");
  });

  it("takes the default reset label from the ui catalog", () => {
    const root = render(Slider, { label: "Smoothing", value: 0.5, defaultValue: 0 });
    const reset = () => get(root, "button").getAttribute("aria-label");
    expect(reset()).toBe("Reset to default");
    setLocale("ja");
    flushSync();
    expect(reset()).toBe("既定値に戻す");
    setLocale("en");
  });

  it("follows the number field and resets to the default", () => {
    const { root, oncommit } = setup();
    const input = get<HTMLInputElement>(root, "input");
    press(input, "ArrowDown");
    expect(get(root, '[role="slider"]').getAttribute("aria-valuenow")).toBe("0.4");
    click(get(root, "button"));
    expect(oncommit).toHaveBeenLastCalledWith(0.3);
    expect(get(root, '[role="slider"]').getAttribute("aria-valuenow")).toBe("0.3");
    expect(input.value).toBe("0.3");
  });
});

// 外から渡した値 (指揮役の値など) を映すだけでは onchange を呼ばない (docs/ui.md#数値の部品と-onchange)
describe("external values", () => {
  function setup(kind: "slider" | "field", initial: number) {
    const onchange = vi.fn();
    const oncommit = vi.fn();
    const control: { set?: (value: number) => void } = {};
    const root = render(ExternalValueHarness, { kind, initial, control, onchange, oncommit });
    const set = (value: number) => {
      control.set?.(value);
      flushSync();
    };
    return { root, onchange, oncommit, set };
  }

  function type(input: HTMLInputElement, text: string) {
    input.focus();
    input.value = text;
    input.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
  }

  it("does not call the slider's callbacks for values off the step grid", () => {
    const { root, onchange, oncommit, set } = setup("slider", 0.43);
    for (const v of [0.57, 0.12, 0.99, 0.31]) set(v);
    const input = get<HTMLInputElement>(root, "input");
    input.focus();
    input.blur();
    flushSync();
    expect(onchange).not.toHaveBeenCalled();
    expect(oncommit).not.toHaveBeenCalled();
    // 外からの値は表示には映る
    expect(input.value).toBe("0.3");
  });

  it("still calls the slider's callbacks for the keyboard, the field and the reset", () => {
    const { root, onchange, oncommit, set } = setup("slider", 0.43);
    const thumb = get(root, '[role="slider"]');
    press(thumb, "ArrowRight");
    expect(onchange).toHaveBeenLastCalledWith(0.5);
    expect(oncommit).toHaveBeenLastCalledWith(0.5);
    set(0.77);
    expect(onchange).toHaveBeenCalledTimes(1);
    const input = get<HTMLInputElement>(root, "input");
    type(input, "0.2");
    press(input, "Enter");
    expect(onchange).toHaveBeenLastCalledWith(0.2);
    set(0.64);
    click(get(root, "button"));
    expect(onchange).toHaveBeenLastCalledWith(0);
    expect(oncommit).toHaveBeenLastCalledWith(0);
    expect(onchange).toHaveBeenCalledTimes(3);
  });

  it("calls the slider's callbacks while dragging, and not after", () => {
    const { root, onchange, oncommit, set } = setup("slider", 0.43);
    const track = get(root, ".ugoe-slider__track");
    // happy-dom では大きさが 0 なので、右へ押せば最大になる
    track.dispatchEvent(
      new PointerEvent("pointerdown", { button: 0, clientX: 10, bubbles: true, cancelable: true }),
    );
    flushSync();
    expect(onchange).toHaveBeenLastCalledWith(1);
    document.dispatchEvent(new PointerEvent("pointerup", { bubbles: true }));
    flushSync();
    expect(oncommit).toHaveBeenLastCalledWith(1);
    const calls = onchange.mock.calls.length;
    set(0.33);
    set(0.48);
    expect(onchange).toHaveBeenCalledTimes(calls);
  });

  it("does not call the number field's onchange for rounding or stale text", () => {
    const { root, onchange, set } = setup("field", 0.43);
    const input = get<HTMLInputElement>(root, "input");
    expect(input.value).toBe("0.4");
    input.focus();
    press(input, "Enter");
    input.blur();
    flushSync();
    // フォーカスがある間も、書き換えていなければ外からの値に追従し、外すときに古い表示を書き込まない
    input.focus();
    set(0.57);
    expect(input.value).toBe("0.6");
    set(0.21);
    input.blur();
    flushSync();
    expect(onchange).not.toHaveBeenCalled();
  });

  it("keeps the user's text against external values and commits it", () => {
    const { root, onchange, set } = setup("field", 0.43);
    const input = get<HTMLInputElement>(root, "input");
    type(input, "0.8");
    set(0.21);
    expect(input.value).toBe("0.8");
    input.blur();
    flushSync();
    expect(onchange).toHaveBeenLastCalledWith(0.8);
    set(0.37);
    press(input, "ArrowUp");
    expect(onchange).toHaveBeenLastCalledWith(0.5);
    expect(onchange).toHaveBeenCalledTimes(2);
  });
});

describe("Select", () => {
  it("is a labelled listbox button that shows the selection", () => {
    const root = render(Select, { label: "Camera", options, value: "b" });
    const trigger = get(root, '[aria-haspopup="listbox"]');
    expect(labelledByText(trigger)).toBe("Camera Beta");
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
  });

  it("shows the placeholder when nothing is selected", () => {
    const root = render(Select, { ariaLabel: "Camera", options, placeholder: "Choose…" });
    const trigger = get(root, '[aria-haspopup="listbox"]');
    expect(trigger.getAttribute("aria-label")).toBe("Camera");
    expect(trigger.textContent?.trim()).toBe("Choose…");
  });

  it("opens with the keyboard and picks an option", async () => {
    const onchange = vi.fn();
    const root = render(Select, { label: "Camera", options, value: "a", onchange });
    const trigger = get(root, '[aria-haspopup="listbox"]');
    trigger.focus();
    press(trigger, "ArrowDown");
    await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
    const items = [...document.querySelectorAll('[role="option"]')];
    expect(items.map((i) => i.textContent?.trim())).toEqual(["Alpha", "Beta", "Gamma", "Delta"]);
    expect(items[2]?.getAttribute("aria-disabled")).toBe("true");
    // 選んでいる項目から下へ移り、Enter で選ぶ
    await vi.waitFor(() => expect(items[0]?.hasAttribute("data-highlighted")).toBe(true));
    press(document.activeElement ?? trigger, "ArrowDown");
    expect(items[1]?.hasAttribute("data-highlighted")).toBe(true);
    press(document.activeElement ?? trigger, "Enter");
    await vi.waitFor(() => expect(onchange).toHaveBeenCalledWith("b"));
    await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("false"));
    expect(labelledByText(trigger)).toBe("Camera Beta");
  });
});

describe("SegmentedControl", () => {
  it("is one tab stop and selects with the arrow keys", () => {
    const onchange = vi.fn();
    const root = render(SegmentedControl, {
      label: "Fit",
      options: options.slice(0, 3),
      value: "a",
      onchange,
    });
    const group = get(root, '[role="radiogroup"]');
    expect(labelledByText(group)).toBe("Fit");
    const radios = [...root.querySelectorAll<HTMLElement>('[role="radio"]')];
    expect(radios.map((r) => r.getAttribute("aria-checked"))).toEqual(["true", "false", "false"]);
    expect(radios.map((r) => r.tabIndex)).toEqual([0, -1, -1]);
    radios[0]?.focus();
    press(radios[0] as HTMLElement, "ArrowRight");
    expect(document.activeElement).toBe(radios[1]);
    expect(onchange).toHaveBeenLastCalledWith("b");
    expect(radios[1]?.getAttribute("aria-checked")).toBe("true");
  });
});

describe("Tabs", () => {
  it("switches panels with the arrow keys", () => {
    const panel = createRawSnippet((value: () => string) => ({
      render: () => `<p>panel ${value()}</p>`,
    }));
    const root = render(Tabs, { tabs: options.slice(0, 2), ariaLabel: "Panels", panel });
    const list = get(root, '[role="tablist"]');
    expect(list.getAttribute("aria-label")).toBe("Panels");
    const tabs = [...root.querySelectorAll<HTMLElement>('[role="tab"]')];
    expect(tabs[0]?.getAttribute("aria-selected")).toBe("true");
    const visible = () => get(root, '[role="tabpanel"]:not([hidden])');
    expect(visible().textContent?.trim()).toBe("panel a");
    expect(visible().getAttribute("aria-labelledby")).toBe(tabs[0]?.id);
    tabs[0]?.focus();
    press(tabs[0] as HTMLElement, "ArrowRight");
    expect(document.activeElement).toBe(tabs[1]);
    expect(tabs[1]?.getAttribute("aria-selected")).toBe("true");
    expect(visible().textContent?.trim()).toBe("panel b");
  });
});

describe("Section", () => {
  it("collapses with a button inside the heading", async () => {
    const onchange = vi.fn();
    const root = render(Section, {
      title: "Lighting",
      children: html("<p>content</p>"),
      onchange,
    });
    const heading = get(root, "h3");
    const trigger = get(heading, "button");
    expect(trigger.textContent?.trim()).toBe("Lighting");
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    const content = document.getElementById(trigger.getAttribute("aria-controls") ?? "");
    expect(content?.textContent).toContain("content");
    click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(onchange).toHaveBeenLastCalledWith(false);
    // 閉じる動きが済んだら中身を隠す
    await vi.waitFor(() => expect(content?.hidden).toBe(true));
  });
});

describe("Toolbar", () => {
  it("is one tab stop with roving focus", () => {
    const root = render(ToolbarHarness, {});
    const toolbar = get(root, '[role="toolbar"]');
    expect(toolbar.getAttribute("aria-label")).toBe("Tools");
    const buttons = [...toolbar.querySelectorAll<HTMLElement>("button")];
    expect(buttons).toHaveLength(3);
    expect(buttons.map((b) => b.tabIndex)).toEqual([0, -1, -1]);
    expect(buttons[0]?.classList).toContain("ugoe-button--ghost");
    expect(buttons[1]?.getAttribute("aria-label")).toBe("Mirror");
    expect(buttons[1]?.getAttribute("aria-pressed")).toBe("true");
    buttons[0]?.focus();
    press(buttons[0] as HTMLElement, "ArrowRight");
    expect(document.activeElement).toBe(buttons[1]);
    press(buttons[1] as HTMLElement, "End");
    expect(document.activeElement).toBe(buttons[2]);
  });

  it("keeps one tab stop when the buttons were disabled at first", async () => {
    const root = render(ToolbarHarness, { disabledAtFirst: true });
    const toolbar = get(root, '[role="toolbar"]');
    const buttons = [...toolbar.querySelectorAll<HTMLElement>("button")];
    click(get(root, "#enable"));
    await vi.waitFor(() => expect(buttons.map((b) => b.tabIndex)).toEqual([0, -1, -1]));
    buttons[0]?.focus();
    press(buttons[0] as HTMLElement, "ArrowRight");
    expect(document.activeElement).toBe(buttons[1]);
    await vi.waitFor(() => expect(buttons.map((b) => b.tabIndex)).toEqual([-1, 0, -1]));
  });
});
