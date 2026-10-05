// @vitest-environment happy-dom
// 「Open base project」で本体を開いた後、モジュールに戻る道 (T1622。docs/editor.md#モジュールに戻る):
// 戻る帯の表示と、戻って適用し直した設定がまた拒否されたときの設定の画面 (docs/testing.md#エディタの部品のテスト)。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import type { Command, ModuleStatus, SlotSummary } from "./ipc";
import ModuleReturnBar from "./ModuleReturnBar.svelte";
import ModuleSettingsDialog from "./ModuleSettingsDialog.svelte";
import { render } from "./viewport/tools/test/harness.svelte";

/** 文言 `text` のボタン。 */
function button(text: string): HTMLButtonElement {
  const found = [...document.querySelectorAll("button")].find(
    (b) => b.textContent?.trim() === text,
  );
  if (!found) throw new Error(`no button ${text}`);
  return found;
}

describe("ModuleReturnBar", () => {
  it("names the module and returns to it", () => {
    const onreturn = vi.fn();
    const ondismiss = vi.fn();
    const { target } = render(ModuleReturnBar, {
      back: { project: "/work/Hair.ugoeproj", name: "Hair", settings: null },
      modified: false,
      busy: false,
      onreturn,
      ondismiss,
    });
    expect(target.textContent).toContain("Editing the base of module Hair");
    expect(target.textContent).toContain("It is reopened on the saved base.");
    expect(target.textContent).not.toContain("unsaved changes");
    button("Return to module").click();
    expect(onreturn).toHaveBeenCalledOnce();
    target
      .querySelector<HTMLButtonElement>('button[aria-label="Stay in the base project"]')
      ?.click();
    expect(ondismiss).toHaveBeenCalledOnce();
  });

  it("asks to save the base first and says the settings are applied again", () => {
    const settings = { op: "set_module_settings" } as unknown as Command;
    const { target } = render(ModuleReturnBar, {
      back: { project: "/work/Hair.ugoeproj", name: "Hair", settings },
      modified: true,
      busy: true,
      onreturn: vi.fn(),
      ondismiss: vi.fn(),
    });
    expect(target.textContent).toContain("module settings that were refused are applied again");
    expect(target.textContent).toContain("The base has unsaved changes. Save them first");
    expect(button("Return to module").disabled).toBe(true);
  });
});

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

const module = {
  name: "Hair",
  base: "/work/Base.ugoeproj",
  slot: slot("Hair"),
  requires: { avatar_level: null, slots: [], parameters: [] },
  base_slots: [slot("Hair"), slot("Hat")],
  base_parameters: ["AngleX", "Wind"],
  base_avatar_level: null,
} as unknown as ModuleStatus;

describe("ModuleSettingsDialog after returning to the module", () => {
  it("fills in the settings that were refused again, with the error and the way back to the base", async () => {
    const command: Command = {
      op: "set_module_settings",
      name: "Bangs",
      slot: "Hat",
      requires: { avatar_level: null, slots: [], parameters: ["Wind"] },
      keep_look: false,
    };
    const error = {
      code: "module.read_only_base",
      args: {},
      message: "base elements are read-only",
    };
    const onopenbase = vi.fn();
    const onapply = vi.fn(async () => {});
    render(ModuleSettingsDialog, {
      open: true,
      module,
      onapply,
      onopenbase,
      onclose: vi.fn(),
      retry: { command, error },
    });
    flushSync();
    expect(document.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("Bangs");
    expect(document.querySelector<HTMLInputElement>('input[value="Hat"]')?.checked).toBe(true);
    expect(document.querySelector('[role="alert"]')?.textContent).toBeTruthy();
    // 案内の「Open base project」は、また拒否された設定を渡す (戻った後に適用し直す)
    button("Open base project").click();
    expect(onopenbase).toHaveBeenCalledExactlyOnceWith(command);
    button("Apply").click();
    await vi.waitFor(() => expect(onapply).toHaveBeenCalledOnce());
    expect(onapply).toHaveBeenCalledWith(command);
  });
});
