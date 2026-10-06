// @vitest-environment happy-dom
// 新しいモジュールとモジュールの設定のダイアログの決定 (T1645。docs/editor.md#ダイアログ。docs/testing.md#エディタの部品のテスト):
// 決定のボタンは `type="submit"`・`form` で中の `<form>` を送り、名前の欄の Enter と同じ 1 つの道で 1 回だけ送る。
// 押せないとき (名前が不正・スロットが無い) はボタンでも Enter でも送らない。
import { flushSync } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { type BaseSummary, ipc, type ModuleStatus, type SlotSummary } from "./ipc";
import ModuleSettingsDialog from "./ModuleSettingsDialog.svelte";
import NewModuleDialog from "./NewModuleDialog.svelte";
import { render } from "./viewport/tools/test/harness.svelte";

afterEach(() => {
  vi.restoreAllMocks();
  delete window.__UGOE_FILE_DIALOGS__;
});

/** 文言 `text` のボタン。 */
function button(text: string): HTMLButtonElement {
  const found = [...document.querySelectorAll("button")].find(
    (b) => b.textContent?.trim() === text,
  );
  if (!found) throw new Error(`no button ${text}`);
  return found;
}

/** 名前の欄。 */
function nameField(): HTMLInputElement {
  const field = document.querySelector<HTMLInputElement>('[role="dialog"] input[type="text"]');
  if (!field) throw new Error("no name field");
  return field;
}

/** 欄に打つ。 */
function type(field: HTMLInputElement, value: string): void {
  field.value = value;
  field.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
}

/**
 * 欄の Enter。ブラウザの implicit submission (既定のボタンがあればそれを押したのと同じで、押せなければ送らない。
 * 無ければ form を送る) を `requestSubmit` で再現する (happy-dom は Enter で送らないため)。
 */
function pressEnter(field: HTMLInputElement): void {
  const form = field.form;
  if (!form) throw new Error("the field is not in a form");
  const submitter = [...form.elements].find(
    (e): e is HTMLButtonElement => e instanceof HTMLButtonElement && e.type === "submit",
  );
  if (submitter?.disabled) return;
  form.requestSubmit(submitter);
}

/** 決定のボタンが `form` で名前の欄と同じ `<form>` を送る形か。 */
function expectSubmitsTheForm(primary: HTMLButtonElement): void {
  expect(primary.type).toBe("submit");
  expect(primary.getAttribute("form")).toBeTruthy();
  expect(primary.form).not.toBeNull();
  expect(primary.form).toBe(nameField().form);
}

function slot(id: string): SlotSummary {
  return {
    id,
    anchor: null,
    part: null,
    draw_order_min: 0,
    draw_order_max: 0,
    replaces: [],
    parameters: [],
    pose_parts: [],
  };
}

describe("NewModuleDialog", () => {
  /** 本体を選んで、スロットと名前の候補が入った状態にする。 */
  async function openWithBase(slots: SlotSummary[]) {
    window.__UGOE_FILE_DIALOGS__ = {
      openFile: async () => "/work/Base.ugoe",
      openDirectory: async () => "/work/Base.ugoeproj",
      saveFile: async () => null,
      defaultDirectory: async () => null,
    };
    const base: BaseSummary = { name: "Base", slots };
    vi.spyOn(ipc, "readModuleBase").mockResolvedValue(base);
    const oncreate = vi.fn(async (_base: string, _slot: string, _name: string) => {});
    render(NewModuleDialog, { open: true, startDir: null, oncreate, onclose: vi.fn() });
    button("Project folder…").click();
    await vi.waitFor(() => expect(document.body.textContent).toContain("/work/Base.ugoeproj"));
    flushSync();
    return oncreate;
  }

  it("creates once with the Create button through the form", async () => {
    const oncreate = await openWithBase([slot("Hat")]);
    const create = button("Create");
    expectSubmitsTheForm(create);
    expect(create.disabled).toBe(false);
    create.click();
    await vi.waitFor(() => expect(oncreate).toHaveBeenCalled());
    expect(oncreate).toHaveBeenCalledExactlyOnceWith("/work/Base.ugoeproj", "Hat", "Hat");
  });

  it("creates once with Enter in the name field", async () => {
    const oncreate = await openWithBase([slot("Hat")]);
    type(nameField(), " Cap ");
    pressEnter(nameField());
    await vi.waitFor(() => expect(oncreate).toHaveBeenCalled());
    expect(oncreate).toHaveBeenCalledExactlyOnceWith("/work/Base.ugoeproj", "Hat", "Cap");
  });

  it("does not create with an invalid name", async () => {
    const oncreate = await openWithBase([slot("Hat")]);
    type(nameField(), "a/b");
    expect(button("Create").disabled).toBe(true);
    pressEnter(nameField());
    button("Create").click();
    // 送ってしまったなら、次のマイクロタスクまでに oncreate が呼ばれている
    await Promise.resolve();
    expect(oncreate).not.toHaveBeenCalled();
  });

  it("does not create when the base has no slots", async () => {
    const oncreate = await openWithBase([]);
    expect(document.body.textContent).toContain("no slots");
    const create = button("Create");
    expect(create.disabled).toBe(true);
    create.click();
    create.form?.requestSubmit();
    await Promise.resolve();
    expect(oncreate).not.toHaveBeenCalled();
  });
});

describe("ModuleSettingsDialog", () => {
  const module = {
    name: "Hair",
    base: "/work/Base.ugoeproj",
    slot: slot("Hair"),
    requires: { avatar_level: null, slots: [], parameters: [] },
    base_slots: [slot("Hair"), slot("Hat")],
    base_parameters: ["AngleX"],
    base_avatar_level: null,
  } as unknown as ModuleStatus;

  function open() {
    const onapply = vi.fn(async () => {});
    render(ModuleSettingsDialog, {
      open: true,
      module,
      onapply,
      onopenbase: vi.fn(),
      onclose: vi.fn(),
    });
    flushSync();
    return onapply;
  }

  it("applies once with the Apply button through the form", async () => {
    const onapply = open();
    const apply = button("Apply");
    expectSubmitsTheForm(apply);
    apply.click();
    await vi.waitFor(() => expect(onapply).toHaveBeenCalled());
    expect(onapply).toHaveBeenCalledOnce();
    expect(onapply).toHaveBeenCalledWith(
      expect.objectContaining({ op: "set_module_settings", name: "Hair", slot: "Hair" }),
    );
  });

  it("applies once with Enter in the name field", async () => {
    const onapply = open();
    type(nameField(), "Bangs");
    pressEnter(nameField());
    await vi.waitFor(() => expect(onapply).toHaveBeenCalled());
    expect(onapply).toHaveBeenCalledOnce();
    expect(onapply).toHaveBeenCalledWith(expect.objectContaining({ name: "Bangs" }));
  });

  it("does not apply with an invalid name", async () => {
    const onapply = open();
    type(nameField(), "#Hair");
    expect(button("Apply").disabled).toBe(true);
    pressEnter(nameField());
    button("Apply").click();
    await Promise.resolve();
    expect(onapply).not.toHaveBeenCalled();
  });
});
