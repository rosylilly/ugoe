// 空を持てる数値の欄・複数行の欄・秘密の欄 (T0949) のキーボード操作と aria の検査。happy-dom の上で描く。

import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import NumberField from "./components/NumberField.svelte";
import SecretField from "./components/SecretField.svelte";
import TextArea from "./components/TextArea.svelte";
import { get, labelledByText, press, render } from "./test/render";
import SecretFieldHarness from "./test/SecretFieldHarness.svelte";

/** 欄に文字を打つ (input を送り、Svelte の更新を流す)。 */
function type(input: HTMLInputElement | HTMLTextAreaElement, text: string) {
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
}

function blur(input: HTMLElement) {
  input.blur();
  flushSync();
}

function describedByText(el: Element): string {
  const ids = el.getAttribute("aria-describedby")?.split(/\s+/) ?? [];
  return ids.map((id) => document.getElementById(id)?.textContent?.trim() ?? "").join(" ");
}

describe("NumberField optional", () => {
  function setup(props: Record<string, unknown> = {}) {
    const onchange = vi.fn();
    const oninput = vi.fn();
    const root = render(NumberField, {
      label: "Fade",
      optional: true,
      value: null,
      min: 0,
      max: 10,
      step: 0.1,
      unit: "s",
      onchange,
      oninput,
      ...props,
    });
    const input = get<HTMLInputElement>(root, "input");
    return { root, input, onchange, oninput };
  }

  it("shows an empty value with the default as the placeholder", () => {
    const { input } = setup({ emptyValue: 0.5 });
    expect(input.value).toBe("");
    expect(input.placeholder).toBe("0.5");
    expect(input.hasAttribute("aria-valuenow")).toBe(false);
    expect(input.hasAttribute("aria-valuetext")).toBe(false);
    expect(labelledByText(input)).toBe("Fade");
  });

  it("prefers its own placeholder", () => {
    const { input } = setup({ emptyValue: 0.5, placeholder: "motion" });
    expect(input.placeholder).toBe("motion");
  });

  it("commits an emptied field as null", () => {
    const { input, onchange } = setup({ value: 2 });
    expect(input.value).toBe("2");
    input.focus();
    type(input, "  ");
    press(input, "Enter");
    expect(onchange).toHaveBeenLastCalledWith(null);
    expect(input.value).toBe("");
    // 空のまま確定し直しても呼ばない
    type(input, "");
    blur(input);
    expect(onchange).toHaveBeenCalledTimes(1);
  });

  it("commits a typed number from empty and clamps it", () => {
    const { input, onchange } = setup();
    input.focus();
    type(input, "12");
    blur(input);
    expect(onchange).toHaveBeenLastCalledWith(10);
    expect(input.value).toBe("10");
  });

  it("does not commit an untouched empty field", () => {
    const { input, onchange } = setup();
    input.focus();
    press(input, "Enter");
    blur(input);
    expect(onchange).not.toHaveBeenCalled();
  });

  it("reverts to empty on Escape and on unreadable input", () => {
    const { input, onchange } = setup();
    input.focus();
    type(input, "3");
    expect(press(input, "Escape").defaultPrevented).toBe(true);
    expect(input.value).toBe("");
    type(input, "abc");
    blur(input);
    expect(input.value).toBe("");
    expect(onchange).not.toHaveBeenCalled();
  });

  it("steps from the value the empty field stands for", () => {
    const { input, onchange } = setup({ emptyValue: 0.5 });
    press(input, "ArrowUp");
    expect(onchange).toHaveBeenLastCalledWith(0.6);
    expect(input.value).toBe("0.6");
  });

  it("steps from zero in range without an empty value", () => {
    const { input, onchange } = setup({ min: 1 });
    press(input, "ArrowDown");
    expect(onchange).toHaveBeenLastCalledWith(1);
  });

  it("reports readable typing, and null for an empty field", () => {
    const { input, oninput } = setup({ value: 2 });
    input.focus();
    type(input, "4");
    expect(oninput).toHaveBeenLastCalledWith(4);
    type(input, "4x");
    expect(oninput).toHaveBeenCalledTimes(1);
    type(input, "-3");
    // 範囲に収めて知らせる
    expect(oninput).toHaveBeenLastCalledWith(0);
    type(input, "");
    expect(oninput).toHaveBeenLastCalledWith(null);
  });

  it("keeps reverting an emptied field when not optional", () => {
    const onchange = vi.fn();
    const input = get<HTMLInputElement>(
      render(NumberField, { label: "Angle", value: 2, onchange }),
      "input",
    );
    input.focus();
    type(input, "");
    blur(input);
    expect(input.value).toBe("2");
    expect(onchange).not.toHaveBeenCalled();
  });
});

describe("TextArea", () => {
  function setup(props: Record<string, unknown> = {}) {
    const oninput = vi.fn();
    const onchange = vi.fn();
    const root = render(TextArea, { label: "Authors", value: "Ann", oninput, onchange, ...props });
    const area = get<HTMLTextAreaElement>(root, "textarea");
    return { root, area, oninput, onchange };
  }

  it("is a labelled multi-line text box with the hint and the error", () => {
    const { root, area } = setup({ rows: 4, hint: "One per line", error: "Too long" });
    expect(area.getAttribute("rows")).toBe("4");
    expect(labelledByText(area)).toBe("Authors");
    expect(get<HTMLLabelElement>(root, "label").htmlFor).toBe(area.id);
    expect(describedByText(area)).toBe("One per line Too long");
    expect(area.getAttribute("aria-invalid")).toBe("true");
    expect(document.getElementById(area.getAttribute("aria-errormessage") ?? "")?.textContent).toBe(
      "Too long",
    );
    expect(area.classList.contains("ugoe-text-field__input")).toBe(true);
  });

  it("keeps Enter as a new line and commits on blur", () => {
    const { area, oninput, onchange } = setup();
    area.focus();
    type(area, "Ann\nBob");
    expect(oninput).toHaveBeenLastCalledWith("Ann\nBob");
    expect(press(area, "Enter").defaultPrevented).toBe(false);
    expect(onchange).not.toHaveBeenCalled();
    blur(area);
    expect(onchange).toHaveBeenCalledWith("Ann\nBob");
  });

  it("commits on Mod+Enter only when the value changed", () => {
    const { area, onchange } = setup();
    area.focus();
    press(area, "Enter", { ctrlKey: true });
    expect(onchange).not.toHaveBeenCalled();
    type(area, "Bob");
    expect(press(area, "Enter", { metaKey: true }).defaultPrevented).toBe(true);
    expect(onchange).toHaveBeenCalledWith("Bob");
    blur(area);
    expect(onchange).toHaveBeenCalledTimes(1);
  });

  it("reverts on Escape and passes Escape on when there is nothing to revert", () => {
    const { area, onchange } = setup();
    area.focus();
    type(area, "Bob");
    expect(press(area, "Escape").defaultPrevented).toBe(true);
    expect(area.value).toBe("Ann");
    expect(press(area, "Escape").defaultPrevented).toBe(false);
    blur(area);
    expect(onchange).not.toHaveBeenCalled();
  });

  it("is disabled and takes an external label", () => {
    const root = render(TextArea, {
      id: "notes",
      "aria-labelledby": "notes-label",
      disabled: true,
    });
    const area = get<HTMLTextAreaElement>(root, "textarea");
    expect(area.id).toBe("notes");
    expect(area.disabled).toBe(true);
    expect(area.getAttribute("aria-labelledby")).toBe("notes-label");
    expect(get(root, ".ugoe-text-field").hasAttribute("data-disabled")).toBe(true);
  });
});

describe("SecretField", () => {
  it("is a labelled password box that does not suggest", () => {
    const root = render(SecretField, { label: "API key", hint: "Stored by the OS" });
    const input = get<HTMLInputElement>(root, "input");
    expect(input.type).toBe("password");
    expect(input.getAttribute("autocomplete")).toBe("off");
    expect(labelledByText(input)).toBe("API key");
    expect(describedByText(input)).toBe("Stored by the OS");
    expect(input.classList.contains("ugoe-text-field__input")).toBe(true);
  });

  it("reports only whether it is filled, and hands the value over on take", () => {
    const control: { field?: SecretField; filled?: () => boolean } = {};
    const root = render(SecretFieldHarness, { control });
    flushSync();
    const input = get<HTMLInputElement>(root, "input");
    expect(control.filled?.()).toBe(false);
    type(input, "   ");
    expect(control.filled?.()).toBe(false);
    type(input, "sk-123");
    expect(control.filled?.()).toBe(true);
    expect(control.field?.take()).toBe("sk-123");
    flushSync();
    expect(input.value).toBe("");
    expect(control.filled?.()).toBe(false);
    type(input, "sk-456");
    control.field?.clear();
    flushSync();
    expect(input.value).toBe("");
    expect(control.filled?.()).toBe(false);
  });

  it("clears on Escape and passes Escape on when empty", () => {
    const control: { field?: SecretField; filled?: () => boolean } = {};
    const input = get<HTMLInputElement>(render(SecretFieldHarness, { control }), "input");
    type(input, "sk-123");
    expect(press(input, "Escape").defaultPrevented).toBe(true);
    expect(input.value).toBe("");
    expect(control.filled?.()).toBe(false);
    expect(press(input, "Escape").defaultPrevented).toBe(false);
  });
});
