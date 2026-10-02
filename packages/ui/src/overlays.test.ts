// 重ねて出す部品 (2 段目) のキーボード操作・フォーカス・aria の検査 (docs/ui.md#アクセシビリティの基準)。happy-dom の上で描く。

import { setLocale } from "@ugoe/i18n";
import { flushSync } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import ColorPicker from "./components/ColorPicker.svelte";
import DropZone from "./components/DropZone.svelte";
import Kbd from "./components/Kbd.svelte";
import Menu from "./components/Menu.svelte";
import Notice from "./components/Notice.svelte";
import Toaster from "./components/Toaster.svelte";
import AlertDialogHarness from "./test/AlertDialogHarness.svelte";
import ContextMenuHarness from "./test/ContextMenuHarness.svelte";
import DialogHarness from "./test/DialogHarness.svelte";
import PopoverHarness from "./test/PopoverHarness.svelte";
import { click, get, html, labelledByText, press, render } from "./test/render";
import TooltipHarness from "./test/TooltipHarness.svelte";
import { dismissToast, showToast, toasts } from "./toast.svelte";
import type { MenuEntry } from "./types";

const icon = html('<svg viewBox="0 0 16 16"></svg>');

/** body の中 (Portal の先も含む) から探す。 */
const $ = <E extends Element = HTMLElement>(selector: string) => get<E>(document, selector);
const focused = () => document.activeElement as HTMLElement | null;

describe("Dialog", () => {
  async function open(props: Record<string, unknown> = {}) {
    const root = render(DialogHarness, props);
    const opener = get(root, "#opener");
    opener.focus();
    click(opener);
    const dialog = await vi.waitFor(() => $('[role="dialog"]'));
    return { root, opener, dialog };
  }

  it("is a labelled modal that takes the focus", async () => {
    const { dialog } = await open();
    expect(labelledByText(dialog)).toBe("Rename");
    const description = document.getElementById(dialog.getAttribute("aria-describedby") ?? "");
    expect(description?.textContent).toBe("Give the layer a new name.");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    // 最初の操作できる要素 (閉じるボタンではなく中身) にフォーカスが移る
    await vi.waitFor(() => expect(focused()?.getAttribute("role")).toBe("spinbutton"));
    const close = get(dialog, ".ugoe-dialog__close");
    expect(close.getAttribute("aria-label")).toBe("Close");
  });

  it("keeps the focus inside with Tab", async () => {
    const { dialog } = await open();
    await vi.waitFor(() => expect(dialog.contains(focused())).toBe(true));
    const close = get(dialog, ".ugoe-dialog__close");
    close.focus();
    press(close, "Tab");
    expect(focused()?.getAttribute("role")).toBe("spinbutton");
    press(focused() as HTMLElement, "Tab", { shiftKey: true });
    expect(focused()).toBe(close);
  });

  it("closes with Escape and gives the focus back", async () => {
    const onclose = vi.fn();
    const { root, opener, dialog } = await open({ onclose });
    await vi.waitFor(() => expect(dialog.contains(focused())).toBe(true));
    press(focused() as HTMLElement, "Escape");
    await vi.waitFor(() => expect(get(root, "#state").textContent).toBe("closed"));
    expect(onclose).toHaveBeenCalledOnce();
    await vi.waitFor(() => expect(document.querySelector('[role="dialog"]')).toBeNull());
    await vi.waitFor(() => expect(focused()).toBe(opener));
  });

  it("lets a number field take Escape first", async () => {
    const { root, dialog } = await open();
    const input = await vi.waitFor(() => {
      const el = get<HTMLInputElement>(dialog, "input");
      expect(focused()).toBe(el);
      return el;
    });
    input.value = "0.5";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    press(input, "Escape");
    expect(input.value).toBe("1");
    expect(get(root, "#state").textContent).toBe("open");
  });

  it("closes from the close button", async () => {
    const onclose = vi.fn();
    const { root, dialog } = await open({ onclose });
    click(get(dialog, ".ugoe-dialog__close"));
    await vi.waitFor(() => expect(get(root, "#state").textContent).toBe("closed"));
    expect(onclose).toHaveBeenCalledOnce();
  });

  it("can only be closed from inside when not dismissible", async () => {
    const onclose = vi.fn();
    const { root, dialog } = await open({ dismissible: false, onclose });
    expect(dialog.querySelector(".ugoe-dialog__close")).toBeNull();
    await vi.waitFor(() => expect(dialog.contains(focused())).toBe(true));
    press(focused() as HTMLElement, "Escape");
    flushSync();
    expect(get(root, "#state").textContent).toBe("open");
    click(get(dialog, "#ok"));
    await vi.waitFor(() => expect(get(root, "#state").textContent).toBe("closed"));
    expect(onclose).not.toHaveBeenCalled();
  });
});

describe("AlertDialog", () => {
  async function open() {
    const onconfirm = vi.fn();
    const oncancel = vi.fn();
    const root = render(AlertDialogHarness, { onconfirm, oncancel });
    click(get(root, "#opener"));
    const dialog = await vi.waitFor(() => $('[role="alertdialog"]'));
    return { root, dialog, onconfirm, oncancel };
  }

  it("puts the focus on cancel", async () => {
    const { dialog } = await open();
    expect(labelledByText(dialog)).toBe("Forget everything?");
    await vi.waitFor(() => expect(focused()?.textContent?.trim()).toBe("Cancel"));
    const buttons = [...dialog.querySelectorAll("button")];
    expect(buttons.map((b) => b.textContent?.trim())).toEqual(["Cancel", "Forget everything"]);
    expect(buttons[1]?.classList).toContain("ugoe-button--danger");
  });

  it("confirms", async () => {
    const { root, dialog, onconfirm, oncancel } = await open();
    click([...dialog.querySelectorAll("button")][1] as HTMLElement);
    await vi.waitFor(() => expect(get(root, "#state").textContent).toBe("closed"));
    expect(onconfirm).toHaveBeenCalledOnce();
    expect(oncancel).not.toHaveBeenCalled();
  });

  it("cancels with Escape", async () => {
    const { root, onconfirm, oncancel } = await open();
    await vi.waitFor(() => expect(focused()?.textContent?.trim()).toBe("Cancel"));
    press(focused() as HTMLElement, "Escape");
    await vi.waitFor(() => expect(get(root, "#state").textContent).toBe("closed"));
    expect(oncancel).toHaveBeenCalledOnce();
    expect(onconfirm).not.toHaveBeenCalled();
  });
});

describe("Notice", () => {
  it("is a status with the tone in words", () => {
    const root = render(Notice, { tone: "warning", title: "Some textures are missing" });
    const notice = get(root, ".ugoe-notice");
    expect(notice.getAttribute("role")).toBe("status");
    expect(get(notice, ".ugoe-visually-hidden").textContent).toBe("Warning:");
    expect(notice.querySelector("button")).toBeNull();
  });

  it("is an alert for errors and can be dismissed", () => {
    const ondismiss = vi.fn();
    const root = render(Notice, { tone: "danger", title: "Could not open", ondismiss });
    const notice = get(root, ".ugoe-notice");
    expect(notice.getAttribute("role")).toBe("alert");
    const close = get(notice, "button");
    expect(close.getAttribute("aria-label")).toBe("Close");
    click(close);
    expect(ondismiss).toHaveBeenCalledOnce();
  });

  it("follows the language", () => {
    const root = render(Notice, { tone: "success", title: "Saved" });
    setLocale("ja");
    flushSync();
    expect(get(root, ".ugoe-visually-hidden").textContent).toBe("完了:");
    setLocale("en");
  });
});

describe("Toast", () => {
  afterEach(() => {
    for (const item of [...toasts()]) dismissToast(item.id);
    vi.useRealTimers();
  });

  it("shows toasts in a status region and drops them after a while", () => {
    vi.useFakeTimers();
    const root = render(Toaster, {});
    const region = get(root, ".ugoe-toaster");
    expect(region.getAttribute("role")).toBe("status");
    expect(region.getAttribute("aria-label")).toBe("Notifications");
    expect(region.getAttribute("popover")).toBe("manual");
    showToast("Saved", { tone: "success", duration: 1000 });
    flushSync();
    const toast = get(region, ".ugoe-toast");
    expect(toast.textContent).toContain("Saved");
    expect(toast.dataset.tone).toBe("success");
    // フォーカスは奪わない
    expect(region.contains(focused())).toBe(false);
    vi.advanceTimersByTime(1000);
    flushSync();
    expect(region.querySelector(".ugoe-toast")).toBeNull();
  });

  it("waits while the pointer is over it", () => {
    vi.useFakeTimers();
    const root = render(Toaster, {});
    const region = get(root, ".ugoe-toaster");
    showToast("Exported", { duration: 1000 });
    flushSync();
    vi.advanceTimersByTime(600);
    region.dispatchEvent(new PointerEvent("pointerenter"));
    vi.advanceTimersByTime(5000);
    flushSync();
    expect(region.querySelectorAll(".ugoe-toast")).toHaveLength(1);
    region.dispatchEvent(new PointerEvent("pointerleave"));
    vi.advanceTimersByTime(399);
    flushSync();
    expect(region.querySelectorAll(".ugoe-toast")).toHaveLength(1);
    vi.advanceTimersByTime(1);
    flushSync();
    expect(region.querySelectorAll(".ugoe-toast")).toHaveLength(0);
  });

  it("closes from its button and can stay", () => {
    const root = render(Toaster, {});
    showToast("Connected", { duration: 0 });
    flushSync();
    const close = get(root, ".ugoe-toast button");
    expect(close.getAttribute("aria-label")).toBe("Close");
    click(close);
    expect(root.querySelector(".ugoe-toast")).toBeNull();
    expect(toasts()).toHaveLength(0);
  });
});

describe("Tooltip", () => {
  it("shows on focus, describes the trigger and hides on Escape", async () => {
    const root = render(TooltipHarness, {});
    const trigger = get(root, "#plain");
    trigger.focus();
    const tip = await vi.waitFor(() => $('[role="tooltip"]'));
    expect(tip.textContent).toContain("Fit the model to the view");
    expect(trigger.getAttribute("aria-describedby")).toBeTruthy();
    expect(
      document.getElementById(trigger.getAttribute("aria-describedby") ?? "")?.textContent,
    ).toContain("Fit the model to the view");
    press(trigger, "Escape");
    await vi.waitFor(() => expect(document.querySelector('[role="tooltip"]')).toBeNull());
  });

  it("is on IconButton with the label and the shortcut, without describing it twice", async () => {
    const root = render(TooltipHarness, {});
    const undo = get(root, "#undo");
    expect(undo.getAttribute("aria-label")).toBe("Undo");
    expect(undo.getAttribute("aria-keyshortcuts")).toMatch(/^(Control|Meta)\+Z$/);
    expect(undo.hasAttribute("title")).toBe(false);
    undo.focus();
    const tip = await vi.waitFor(() => $('[role="tooltip"]'));
    expect(tip.textContent).toContain("Undo");
    expect(tip.querySelector("kbd")).not.toBeNull();
    expect(undo.hasAttribute("aria-describedby")).toBe(false);
    undo.blur();
    await vi.waitFor(() => expect(document.querySelector('[role="tooltip"]')).toBeNull());
    get(root, "#quiet").focus();
    flushSync();
    await new Promise((r) => setTimeout(r, 20));
    expect(document.querySelector('[role="tooltip"]')).toBeNull();
  });
});

describe("Menu", () => {
  function setup(extra: Record<string, unknown> = {}) {
    const onsave = vi.fn();
    const onwrap = vi.fn();
    const ondelete = vi.fn();
    const entries: MenuEntry[] = [
      { label: "Save", shortcut: "Mod+S", onselect: onsave },
      { label: "Export", onselect: () => {}, disabled: true },
      { kind: "separator" },
      { kind: "checkbox", label: "Wrap", checked: false, onchange: onwrap },
      { label: "Delete", danger: true, onselect: ondelete },
    ];
    const root = render(Menu, { label: "File", entries, ...extra });
    const trigger = get(root, "button");
    return { root, trigger, onsave, onwrap, ondelete };
  }

  async function openWithKeyboard(trigger: HTMLElement) {
    trigger.focus();
    press(trigger, "ArrowDown");
    await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
    return $('[role="menu"]');
  }

  it("is a menu button", () => {
    const { trigger } = setup();
    expect(trigger.textContent?.trim()).toBe("File");
    expect(trigger.getAttribute("aria-haspopup")).toBe("menu");
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
  });

  it("opens with the keyboard and runs an item", async () => {
    const { trigger, onsave } = setup();
    const menu = await openWithKeyboard(trigger);
    const items = [...menu.querySelectorAll<HTMLElement>('[role^="menuitem"]')];
    expect(items.map((i) => i.getAttribute("role"))).toEqual([
      "menuitem",
      "menuitem",
      "menuitemcheckbox",
      "menuitem",
    ]);
    expect(items[0]?.getAttribute("aria-keyshortcuts")).toMatch(/^(Control|Meta)\+S$/);
    expect(items[0]?.querySelector('kbd[aria-hidden="true"]')?.textContent).toMatch(
      /^(Ctrl\+S|⌘S)$/,
    );
    expect(items[1]?.hasAttribute("data-disabled")).toBe(true);
    expect(items[2]?.getAttribute("aria-checked")).toBe("false");
    await vi.waitFor(() => expect(focused()).toBe(items[0]));
    press(items[0] as HTMLElement, "Enter");
    await vi.waitFor(() => expect(onsave).toHaveBeenCalledOnce());
    await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("false"));
    await vi.waitFor(() => expect(focused()).toBe(trigger));
  });

  it("skips disabled items and toggles checkbox items", async () => {
    const { trigger, onwrap } = setup();
    const menu = await openWithKeyboard(trigger);
    const items = [...menu.querySelectorAll<HTMLElement>('[role^="menuitem"]')];
    await vi.waitFor(() => expect(focused()).toBe(items[0]));
    press(items[0] as HTMLElement, "ArrowDown");
    expect(focused()).toBe(items[2]);
    press(items[2] as HTMLElement, "Enter");
    await vi.waitFor(() => expect(onwrap).toHaveBeenCalledWith(true));
  });

  it("closes with Escape", async () => {
    const { trigger, onsave } = setup();
    await openWithKeyboard(trigger);
    press(focused() as HTMLElement, "Escape");
    await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("false"));
    expect(onsave).not.toHaveBeenCalled();
    await vi.waitFor(() => expect(focused()).toBe(trigger));
  });

  it("can be an icon button with a tooltip", async () => {
    const { trigger } = setup({ icon });
    expect(trigger.getAttribute("aria-label")).toBe("File");
    expect(trigger.getAttribute("aria-haspopup")).toBe("menu");
    await openWithKeyboard(trigger);
    press(focused() as HTMLElement, "Escape");
    await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("false"));
  });
});

describe("ContextMenu", () => {
  it("opens on the context menu event", async () => {
    const onselect = vi.fn();
    const root = render(ContextMenuHarness, { entries: [{ label: "Duplicate", onselect }] });
    const area = get(root, "#area");
    area.dispatchEvent(
      new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 10, clientY: 10 }),
    );
    const item = await vi.waitFor(() => $('[role="menu"] [role="menuitem"]'));
    expect(item.textContent?.trim()).toBe("Duplicate");
    click(item);
    await vi.waitFor(() => expect(onselect).toHaveBeenCalledOnce());
  });
});

describe("Popover", () => {
  it("opens from its trigger and closes with Escape back to it", async () => {
    const root = render(PopoverHarness, {});
    const trigger = get(root, "#trigger");
    expect(trigger.getAttribute("aria-haspopup")).toBe("dialog");
    trigger.focus();
    click(trigger);
    await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
    const content = $(".ugoe-popover");
    expect(content.getAttribute("aria-label")).toBe("Light settings");
    expect(trigger.getAttribute("aria-controls")).toBe(content.id);
    await vi.waitFor(() => expect(content.contains(focused())).toBe(true));
    press(focused() as HTMLElement, "Escape");
    await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("false"));
    await vi.waitFor(() => expect(focused()).toBe(trigger));
  });
});

describe("ColorPicker", () => {
  async function setup() {
    const onchange = vi.fn();
    const root = render(ColorPicker, { label: "Light color", value: "#FF8800", onchange });
    const trigger = get(root, "button");
    click(trigger);
    const panel = await vi.waitFor(() => $(".ugoe-popover"));
    return { root, trigger, panel, onchange };
  }

  it("names the trigger with the label and the value", async () => {
    const { trigger, panel } = await setup();
    expect(labelledByText(trigger)).toBe("Light color #ff8800");
    expect(panel.getAttribute("aria-label")).toBe("Light color");
    const native = get<HTMLInputElement>(panel, 'input[type="color"]');
    expect(native.value).toBe("#ff8800");
    expect(native.closest("label")?.textContent).toContain("System picker");
    const fields = [...panel.querySelectorAll('[role="spinbutton"]')];
    expect(fields.map((f) => labelledByText(f))).toEqual(["Red", "Green", "Blue"]);
    expect(fields.map((f) => f.getAttribute("aria-valuenow"))).toEqual(["255", "136", "0"]);
  });

  it("takes a typed hex value and the channels", async () => {
    const { trigger, panel, onchange } = await setup();
    const hex = get<HTMLInputElement>(panel, 'input[type="text"]:not([role])');
    hex.focus();
    hex.value = "#0af";
    hex.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    press(hex, "Enter");
    expect(onchange).toHaveBeenLastCalledWith("#00aaff");
    expect(labelledByText(trigger)).toBe("Light color #00aaff");
    const red = get(panel, '[role="spinbutton"]');
    press(red, "ArrowUp");
    expect(onchange).toHaveBeenLastCalledWith("#01aaff");
  });

  it("reverts an unreadable hex value", async () => {
    const { panel, onchange } = await setup();
    const hex = get<HTMLInputElement>(panel, 'input[type="text"]:not([role])');
    hex.focus();
    hex.value = "orange";
    hex.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    press(hex, "Enter");
    expect(hex.value).toBe("#ff8800");
    expect(onchange).not.toHaveBeenCalled();
  });
});

describe("DropZone", () => {
  function drop(target: Element, files: File[]) {
    // happy-dom の DataTransfer は files を持たないので、ドロップで使うところだけを作る
    const event = new Event("drop", { bubbles: true, cancelable: true });
    Object.defineProperty(event, "dataTransfer", { value: { files, types: ["Files"] } });
    target.dispatchEvent(event);
    flushSync();
    return event;
  }

  it("has a button that does the same as dropping", () => {
    const onfiles = vi.fn();
    const root = render(DropZone, { accept: ".ugoe", onfiles });
    const zone = get(root, ".ugoe-drop-zone");
    expect(labelledByText(zone)).toBe("Drop a file here");
    const button = get(zone, "button");
    expect(button.textContent?.trim()).toBe("Choose file…");
    const input = get<HTMLInputElement>(zone, 'input[type="file"]');
    expect(input.accept).toBe(".ugoe");
    expect(input.tabIndex).toBe(-1);
    const picked = vi.spyOn(input, "click").mockImplementation(() => {});
    click(button);
    expect(picked).toHaveBeenCalledOnce();
  });

  it("sorts dropped files by accept", () => {
    const onfiles = vi.fn();
    const onreject = vi.fn();
    const root = render(DropZone, { accept: ".ugoe,image/*", multiple: true, onfiles, onreject });
    const zone = get(root, ".ugoe-drop-zone");
    const model = new File(["x"], "Model.UGOE");
    const image = new File(["x"], "face.png", { type: "image/png" });
    const text = new File(["x"], "notes.txt", { type: "text/plain" });
    const event = drop(zone, [model, image, text]);
    expect(event.defaultPrevented).toBe(true);
    expect(onfiles).toHaveBeenCalledWith([model, image]);
    expect(onreject).toHaveBeenCalledWith([text]);
  });

  it("keeps one file unless multiple", () => {
    const onfiles = vi.fn();
    const onreject = vi.fn();
    const root = render(DropZone, { onfiles, onreject });
    const a = new File(["a"], "a.ugoe");
    const b = new File(["b"], "b.ugoe");
    drop(get(root, ".ugoe-drop-zone"), [a, b]);
    expect(onfiles).toHaveBeenCalledWith([a]);
    expect(onreject).toHaveBeenCalledWith([b]);
  });

  it("shows the dragging state and can use the app's own file dialog", () => {
    const onbrowse = vi.fn();
    const root = render(DropZone, { active: true, onbrowse, prompt: "Drop a model" });
    const zone = get(root, ".ugoe-drop-zone");
    expect(zone.hasAttribute("data-dragging")).toBe(true);
    expect(zone.querySelector('input[type="file"]')).toBeNull();
    click(get(zone, "button"));
    expect(onbrowse).toHaveBeenCalledOnce();
    expect(labelledByText(zone)).toBe("Drop a model");
  });
});

describe("Kbd", () => {
  it("writes shortcuts for macOS and the others", () => {
    const mac = render(Kbd, { keys: "Mod+Shift+Z", platform: "mac" });
    expect([...mac.querySelectorAll("kbd kbd")].map((k) => k.textContent)).toEqual(["⇧", "⌘", "Z"]);
    expect(get(mac, ".ugoe-kbd").textContent).toBe("⇧⌘Z");
    const other = render(Kbd, { keys: "Mod+Shift+Z", platform: "other" });
    expect([...other.querySelectorAll("kbd kbd")].map((k) => k.textContent)).toEqual([
      "Ctrl",
      "Shift",
      "Z",
    ]);
    expect(get(other, ".ugoe-kbd").textContent).toBe("Ctrl+Shift+Z");
  });
});
